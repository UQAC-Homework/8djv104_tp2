use crate::application::events::event::UserEvent;
use std::collections::VecDeque;

/// Structure used to queue [UserEvent]
pub struct EventQueue {
    queue: VecDeque<UserEvent>,
}

impl EventQueue {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    /// Adds the given event to the queue
    pub fn push(&mut self, event: UserEvent) {
        self.queue.push_back(event);
    }
    
    /// Attempts to remove an event from the queue
    pub fn pull(&mut self) -> Option<UserEvent> {
        self.queue.pop_front()
    }
}
