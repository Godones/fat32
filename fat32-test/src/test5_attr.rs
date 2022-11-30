use core::fmt::Debug;
use fat32_trait::DirectoryLike;
use std::error::Error;
use std::sync::Arc;

pub fn test_attr(root: Arc<dyn DirectoryLike<Error: Error + Debug>>) {}
