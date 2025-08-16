use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::info;

#[derive(Clone)]
pub struct ShutdownSignal {
    tx: Arc<broadcast::Sender<()>>,
    rx: Arc<broadcast::Receiver<()>>,
}

impl ShutdownSignal {
    pub fn new() -> Self {
        let (tx, rx) = broadcast::channel(1);
        Self {
            tx: Arc::new(tx),
            rx: Arc::new(rx),
        }
    }

    pub async fn wait(&self) {
        let mut rx = self.rx.resubscribe();
        let _ = rx.recv().await;
        info!("Shutdown signal received");
    }

    pub fn shutdown(&self) {
        let _ = self.tx.send(());
    }

    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.rx.resubscribe()
    }
}

impl Default for ShutdownSignal {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_shutdown_signal() {
        let shutdown = ShutdownSignal::new();
        let shutdown_clone = shutdown.clone();

        // Spawn a task that waits for shutdown
        let handle = tokio::spawn(async move {
            shutdown_clone.wait().await;
        });

        // Send shutdown signal
        shutdown.shutdown();

        // Wait for the task to complete
        let _ = handle.await;
    }
} 