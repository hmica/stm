use crossterm::event::{Event as CrosstermEvent, EventStream, KeyEventKind};
use futures::StreamExt;
use std::time::Duration;
use tokio::sync::mpsc;

/// Events produced by the event handler.
pub enum Event {
    Tick,
    Key(crossterm::event::KeyEvent),
    Resize,
}

pub struct EventHandler {
    rx: mpsc::UnboundedReceiver<Event>,
    _task: tokio::task::JoinHandle<()>,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        let task = tokio::spawn(async move {
            let mut reader = EventStream::new();
            let mut tick_interval = tokio::time::interval(tick_rate);

            loop {
                tokio::select! {
                    _ = tick_interval.tick() => {
                        if tx.send(Event::Tick).is_err() {
                            break;
                        }
                    }
                    Some(Ok(event)) = reader.next() => {
                        let mapped = match event {
                            CrosstermEvent::Key(key) if key.kind == KeyEventKind::Press => {
                                Some(Event::Key(key))
                            }
                            CrosstermEvent::Resize(_, _) => Some(Event::Resize),
                            _ => None,
                        };
                        if let Some(ev) = mapped {
                            if tx.send(ev).is_err() {
                                break;
                            }
                        }
                    }
                }
            }
        });

        Self { rx, _task: task }
    }

    pub async fn next(&mut self) -> Option<Event> {
        self.rx.recv().await
    }
}
