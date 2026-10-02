// crates/client/public/sw.js (добавить/обновить)

self.addEventListener('message', (event) => {
  if (event.data && event.data.type === 'QUEUE_UPDATE') {
    const queueName = event.data.queue; // "student" или "admin"
    console.log(`[SW] Queue update: ${queueName}`);
    
    if (navigator.onLine) {
      // Разные теги для разных очередей — можно обрабатывать независимо
      if ('SyncManager' in self) {
        event.waitUntil(
          self.registration.sync.register(`sync-${queueName}-queue`)
        );
      }
    }
  }
});

self.addEventListener('sync', (event) => {
  // Обработка конкретной очереди
  const match = event.tag.match(/^sync-(.+)-queue$/);
  if (match) {
    const queueName = match[1];
    console.log(`[SW] Background sync for queue: ${queueName}`);
    
    event.waitUntil(
      self.clients.matchAll().then(clients => {
        clients.forEach(client => {
          client.postMessage({ 
            type: 'PROCESS_QUEUE', 
            queue: queueName 
          });
        });
      })
    );
  }
});