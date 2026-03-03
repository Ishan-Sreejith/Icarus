use std::any::Any;
use std::fmt;

/// Python-style dynamic collection with heap allocation
#[allow(dead_code)]
pub struct DynamicList {
    items: Vec<Box<dyn Any>>,
    capacity: usize,
}

#[allow(dead_code)]
impl DynamicList {
    pub fn new() -> Self {
        DynamicList::with_capacity(8)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        DynamicList {
            items: Vec::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push<T: Any + 'static>(&mut self, item: T) {
        self.items.push(Box::new(item));

        // Slab allocation: grow in chunks
        if self.items.len() >= self.capacity {
            self.capacity *= 2;
            self.items.reserve(self.capacity - self.items.len());
        }
    }

    pub fn get<T: Any + 'static>(&self, index: usize) -> Option<&T> {
        self.items.get(index)?.downcast_ref::<T>()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }
}

impl fmt::Debug for DynamicList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DynamicList[len={}, cap={}]",
            self.items.len(),
            self.capacity
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dynamic_list() {
        let mut list = DynamicList::new();
        list.push(42i32);
        list.push("hello".to_string());
        list.push(3.14f64);

        assert_eq!(list.len(), 3);
        assert_eq!(list.get::<i32>(0), Some(&42));
        assert_eq!(list.get::<String>(1), Some(&"hello".to_string()));
        assert_eq!(list.get::<f64>(2), Some(&3.14));
    }
}
