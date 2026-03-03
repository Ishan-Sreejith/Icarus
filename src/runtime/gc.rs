use std::collections::HashMap;
use std::fs::File;

/// Resource garbage collector for automatic cleanup
#[allow(dead_code)]
pub struct ResourceGC {
    file_handles: HashMap<String, File>,
    scopes: Vec<Vec<String>>,
}

#[allow(dead_code)]
impl ResourceGC {
    pub fn new() -> Self {
        ResourceGC {
            file_handles: HashMap::new(),
            scopes: vec![Vec::new()],
        }
    }

    pub fn open_file(&mut self, name: String, path: &str) -> std::io::Result<()> {
        let file = File::open(path)?;
        self.file_handles.insert(name.clone(), file);

        // Track in current scope
        if let Some(scope) = self.scopes.last_mut() {
            scope.push(name);
        }

        Ok(())
    }

    pub fn close_file(&mut self, name: &str) {
        self.file_handles.remove(name);
    }

    pub fn enter_scope(&mut self) {
        self.scopes.push(Vec::new());
    }

    pub fn exit_scope(&mut self) {
        if let Some(scope) = self.scopes.pop() {
            // Auto-close all files in this scope
            for name in scope {
                self.file_handles.remove(&name);
            }
        }
    }

    pub fn cleanup_all(&mut self) {
        self.file_handles.clear();
        self.scopes.clear();
    }
}

impl Drop for ResourceGC {
    fn drop(&mut self) {
        // Ensure all resources are cleaned up
        self.cleanup_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_resource_gc() {
        // Create a test file
        fs::write("/tmp/test_gc.txt", "test").unwrap();

        let mut gc = ResourceGC::new();

        gc.enter_scope();
        gc.open_file("f1".to_string(), "/tmp/test_gc.txt").unwrap();

        assert_eq!(gc.file_handles.len(), 1);

        gc.exit_scope();

        // File should be auto-closed
        assert_eq!(gc.file_handles.len(), 0);

        // Cleanup
        fs::remove_file("/tmp/test_gc.txt").ok();
    }
}
