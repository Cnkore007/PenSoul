use std::collections::VecDeque;

use crate::lock::Operation;

pub struct OperationQueue {
    queue: VecDeque<Operation>,
}

impl Default for OperationQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl OperationQueue {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    pub fn enqueue(&mut self, op: Operation) {
        self.queue.push_back(op);
    }

    pub fn dequeue(&mut self) -> Option<Operation> {
        self.queue.pop_front()
    }

    pub fn peek(&self) -> Option<&Operation> {
        self.queue.front()
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lock::{OperationStatus, OperationType};

    fn make_op(chapter: &str) -> Operation {
        Operation {
            op_id: "op-1".to_string(),
            op_type: OperationType::UserEdit,
            chapter_id: chapter.to_string(),
            content: "hello".to_string(),
            expected_version: 1,
            timestamp: 1000,
            status: OperationStatus::Pending,
            actual_version: None,
        }
    }

    #[test]
    fn test_enqueue_dequeue() {
        let mut q = OperationQueue::new();
        assert!(q.is_empty());
        assert_eq!(q.len(), 0);

        q.enqueue(make_op("ch1"));
        q.enqueue(make_op("ch2"));
        assert_eq!(q.len(), 2);
        assert!(!q.is_empty());

        let first = q.dequeue().unwrap();
        assert_eq!(first.chapter_id, "ch1");

        let second = q.dequeue().unwrap();
        assert_eq!(second.chapter_id, "ch2");

        assert!(q.dequeue().is_none());
    }

    #[test]
    fn test_peek() {
        let mut q = OperationQueue::new();
        assert!(q.peek().is_none());

        q.enqueue(make_op("ch1"));
        assert_eq!(q.peek().unwrap().chapter_id, "ch1");
        assert_eq!(q.len(), 1); // peek doesn't consume
    }
}
