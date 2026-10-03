// crates/client/src/shared/storage.rs
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::{
    IdbCursorWithValue, IdbDatabase, IdbIndexParameters, IdbObjectStoreParameters,
    IdbOpenDbRequest, IdbRequest, IdbTransaction, IdbTransactionMode,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XapiStatement {
    pub id: String,
    pub timestamp: String,
    pub stored_at: Option<String>,
    pub actor: String,
    pub verb: String,
    pub object: String,
    pub queue_name: String,
    pub retry_count: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientClock {
    pub key: String,
    pub delta_ms: i64,
    pub measured_at: String,
    pub source: String,
}

pub struct OfflineStorage {
    db: IdbDatabase,
}

fn request_to_promise(req: &IdbRequest) -> Result<js_sys::Promise, JsValue> {
    Ok(js_sys::Promise::new(&mut |resolve, reject| {
        let resolve = resolve.clone();
        let reject = reject.clone();
        
        let success_cb = Closure::wrap(Box::new(move |event: web_sys::Event| {
            let target = event.target().unwrap();
            let req = target.dyn_into::<IdbRequest>().unwrap();
            let _ = resolve.call1(&JsValue::NULL, &req.result().unwrap());
        }) as Box<dyn FnMut(_)>);
        
        let error_cb = Closure::wrap(Box::new(move |event: web_sys::Event| {
            let target = event.target().unwrap();
            let req = target.dyn_into::<IdbRequest>().unwrap();
            let err_msg = match req.error() {
                Ok(Some(e)) => e.message(),
                _ => "Unknown error".to_string(),
            };
            let _ = reject.call1(&JsValue::NULL, &JsValue::from_str(&err_msg));
        }) as Box<dyn FnMut(_)>);

        req.set_onsuccess(Some(success_cb.as_ref().unchecked_ref()));
        req.set_onerror(Some(error_cb.as_ref().unchecked_ref()));
        
        success_cb.forget();
        error_cb.forget();
    }))
}

fn tx_complete_promise(tx: &IdbTransaction) -> Result<js_sys::Promise, JsValue> {
    Ok(js_sys::Promise::new(&mut |resolve, reject| {
        let resolve = resolve.clone();
        let reject = reject.clone();
        
        let success_cb = Closure::wrap(Box::new(move |_event: web_sys::Event| {
            let _ = resolve.call0(&JsValue::NULL);
        }) as Box<dyn FnMut(_)>);
        
        let error_cb = Closure::wrap(Box::new(move |event: web_sys::Event| {
            let target = event.target().unwrap();
            let tx = target.dyn_into::<IdbTransaction>().unwrap();
            let err_msg = match tx.error() {
                Some(e) => e.message(),
                None => "Unknown error".to_string(),
            };
            let _ = reject.call1(&JsValue::NULL, &JsValue::from_str(&err_msg));
        }) as Box<dyn FnMut(_)>);

        tx.set_oncomplete(Some(success_cb.as_ref().unchecked_ref()));
        tx.set_onerror(Some(error_cb.as_ref().unchecked_ref()));
        
        success_cb.forget();
        error_cb.forget();
    }))
}

impl OfflineStorage {
    pub async fn new() -> Result<Self, JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let indexed_db = window.indexed_db()?.ok_or("No indexed_db")?;
        
        let req = indexed_db.open_with_u32("lms_offline_db", 3)?;
        
        let state = Rc::new(RefCell::new(None));
        let state_clone = state.clone();
        
        let upgrade_cb = Closure::wrap(Box::new(move |event: web_sys::Event| {
            let target = event.target().unwrap();
            let req = target.dyn_into::<IdbOpenDbRequest>().unwrap();
            let db = req.result().unwrap().dyn_into::<IdbDatabase>().unwrap();
            
            // Создаем offline_xapi_statements
            let store_params = IdbObjectStoreParameters::new();
            store_params.set_key_path(&JsValue::from_str("id"));
            
            if let Ok(store) = db.create_object_store_with_optional_parameters("offline_xapi_statements", &store_params) {
                let index_params = IdbIndexParameters::new();
                index_params.set_unique(false);
                let _ = store.create_index_with_str_and_optional_parameters("timestamp", "timestamp", &index_params);
                leptos::logging::log!("[IndexedDB] Created offline_xapi_statements store");
            }

            // Создаем client_clock
            let clock_params = IdbObjectStoreParameters::new();
            clock_params.set_key_path(&JsValue::from_str("key"));
            if db.create_object_store_with_optional_parameters("client_clock", &clock_params).is_ok() {
                leptos::logging::log!("[IndexedDB] Created client_clock store");
            }
        }) as Box<dyn FnMut(_)>);
        
        let success_cb = Closure::wrap(Box::new(move |event: web_sys::Event| {
            let target = event.target().unwrap();
            let req = target.dyn_into::<IdbOpenDbRequest>().unwrap();
            let db = req.result().unwrap().dyn_into::<IdbDatabase>().unwrap();
            *state_clone.borrow_mut() = Some(Ok(db));
        }) as Box<dyn FnMut(_)>);
        
        let state_clone2 = state.clone();
        let error_cb = Closure::wrap(Box::new(move |event: web_sys::Event| {
            let target = event.target().unwrap();
            let req = target.dyn_into::<IdbOpenDbRequest>().unwrap();
            let err_msg = match req.error() {
                Ok(Some(e)) => e.message(),
                _ => "Unknown error".to_string(),
            };
            *state_clone2.borrow_mut() = Some(Err(JsValue::from_str(&err_msg)));
        }) as Box<dyn FnMut(_)>);

        req.set_onupgradeneeded(Some(upgrade_cb.as_ref().unchecked_ref()));
        req.set_onsuccess(Some(success_cb.as_ref().unchecked_ref()));
        req.set_onerror(Some(error_cb.as_ref().unchecked_ref()));
        
        upgrade_cb.forget();
        success_cb.forget();
        error_cb.forget();

        loop {
            if let Some(result) = state.borrow_mut().take() {
                let db = result?;
                return Ok(Self { db });
            }
            gloo::timers::future::TimeoutFuture::new(10).await;
        }
    }

    pub async fn save_statement(&self, statement: &XapiStatement) -> Result<(), JsValue> {
        let tx = self.db.transaction_with_str_and_mode("offline_xapi_statements", IdbTransactionMode::Readwrite)?;
        let store = tx.object_store("offline_xapi_statements")?;
        
        let js_value = serde_wasm_bindgen::to_value(statement)?;
        let req = store.put(&js_value)?; 
        
        wasm_bindgen_futures::JsFuture::from(request_to_promise(&req)?).await?;
        wasm_bindgen_futures::JsFuture::from(tx_complete_promise(&tx)?).await?;
        Ok(())
    }

    pub async fn get_statements_chunked(&self, limit: usize) -> Result<Vec<XapiStatement>, JsValue> {
        let tx = self.db.transaction_with_str_and_mode("offline_xapi_statements", IdbTransactionMode::Readonly)?;
        let store = tx.object_store("offline_xapi_statements")?;
        
        let mut result = Vec::new();
        let cursor_req = if let Ok(index) = store.index("timestamp") {
            index.open_cursor()?
        } else {
            store.open_cursor()?
        };
        
        let mut cursor_opt: Option<IdbCursorWithValue> = {
            let promise = request_to_promise(&cursor_req)?;
            wasm_bindgen_futures::JsFuture::from(promise).await?.dyn_into().ok()
        };
        
        while let Some(cursor) = cursor_opt {
            if result.len() >= limit { break; }
            let js_val = cursor.value()?;
            if let Ok(statement) = serde_wasm_bindgen::from_value::<XapiStatement>(js_val) {
                result.push(statement);
            }
            #[allow(deprecated)]
            let next_req = { cursor.continue_()?; cursor.request() };
            cursor_opt = {
                let promise = request_to_promise(&next_req)?;
                wasm_bindgen_futures::JsFuture::from(promise).await?.dyn_into().ok()
            };
        }
        Ok(result)
    }

    pub async fn delete_statements(&self, ids: &[String]) -> Result<(), JsValue> {
        if ids.is_empty() {
            return Ok(());
        }
        
        let tx = self.db.transaction_with_str_and_mode("offline_xapi_statements", IdbTransactionMode::Readwrite)?;
        let store = tx.object_store("offline_xapi_statements")?;
        
        for id in ids {
            let req = store.delete(&JsValue::from_str(id))?;
            let _ = wasm_bindgen_futures::JsFuture::from(request_to_promise(&req)?).await;
        }
        
        wasm_bindgen_futures::JsFuture::from(tx_complete_promise(&tx)?).await?;
        Ok(())
    }

    pub async fn save_client_clock(&self, clock: &ClientClock) -> Result<(), JsValue> {
        let tx = self.db.transaction_with_str_and_mode("client_clock", IdbTransactionMode::Readwrite)?;
        let store = tx.object_store("client_clock")?;
        let js_value = serde_wasm_bindgen::to_value(clock)?;
        let req = store.put(&js_value)?;
        wasm_bindgen_futures::JsFuture::from(request_to_promise(&req)?).await?;
        wasm_bindgen_futures::JsFuture::from(tx_complete_promise(&tx)?).await?;
        Ok(())
    }

    pub async fn get_client_clock(&self) -> Result<Option<ClientClock>, JsValue> {
        let tx = self.db.transaction_with_str_and_mode("client_clock", IdbTransactionMode::Readonly)?;
        let store = tx.object_store("client_clock")?;
        let req = store.get(&JsValue::from_str("delta"))?;
        let result = wasm_bindgen_futures::JsFuture::from(request_to_promise(&req)?).await?;
        
        if result.is_null() || result.is_undefined() {
            Ok(None)
        } else {
            Ok(Some(serde_wasm_bindgen::from_value(result)?))
        }
    }
}

static STORAGE: Mutex<Option<Arc<OfflineStorage>>> = Mutex::new(None);

pub async fn get_storage() -> Result<Arc<OfflineStorage>, JsValue> {
    if let Some(storage) = STORAGE.lock().unwrap().clone() {
        return Ok(storage);
    }
    let storage = Arc::new(OfflineStorage::new().await?);
    *STORAGE.lock().unwrap() = Some(storage.clone());
    Ok(storage)
}