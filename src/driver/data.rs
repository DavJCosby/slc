use core::{any::Any, fmt::Debug};

use alloc::collections::BTreeMap;
use compact_str::{CompactString, ToCompactString};

use crate::SledError;

#[derive(Debug)]
struct DataWrapper<T>(T);

impl<T> DataWrapper<T> {
    pub fn new(value: T) -> Self {
        DataWrapper(value)
    }
}

trait Downcastable: Debug {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
impl<T: StorableData + Debug> Downcastable for DataWrapper<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub trait StorableData: 'static + Debug {}
impl<T: Sized + 'static + Debug> StorableData for T {}

#[derive(Debug)]
pub struct Data {
    data: BTreeMap<CompactString, Box<dyn Downcastable>>,
}

impl Data {
    pub fn new() -> Self {
        Data {
            data: BTreeMap::new(),
        }
    }

    /// Returns `Ok(&T)` if some data of type `T` is associated with the given `key`. Otherwise, returns an error.
    /// ```rust
    /// # use spatial_led::{SledError, driver::{Data}};
    /// # pub fn main() -> Result<(), SledError> {
    ///     let mut data = Data::new();
    ///     data.set("abc", 123);
    ///
    ///     let retrieved: &i32 = data.get("abc")?;
    ///     assert_eq!(retrieved, &123);
    ///
    ///     let type_mismatch = data.get::<bool>("abc");
    ///     assert_eq!(
    ///         &type_mismatch.err().unwrap().message,
    ///         "Data associated with the key `abc` exists, but it is not of type bool."
    ///     );
    ///
    ///     let bad_key = data.get::<i32>("cba");
    ///     assert_eq!(
    ///         &bad_key.err().unwrap().message,
    ///         "No data associated with the key `cba`."
    ///     );
    /// #   Ok(())
    /// # }
    /// ```
    pub fn get<T: StorableData>(&self, key: &str) -> Result<&T, SledError> {
        let candidate = self
            .data
            .get(key)
            .ok_or_else(|| SledError::new(format!("No data associated with the key `{}`.", key)))?;

        match candidate.as_any().downcast_ref::<DataWrapper<T>>() {
            Some(wrapper) => Ok(&wrapper.0),
            None => Err(SledError::new(format!(
                "Data associated with the key `{}` exists, but it is not of type {}.",
                key,
                core::any::type_name::<T>()
            ))),
        }
    }

    pub fn get_mut<T: StorableData>(&mut self, key: &str) -> Result<&mut T, SledError> {
        let candidate = self
            .data
            .get_mut(key)
            .ok_or_else(|| SledError::new(format!("No data associated with the key `{}`.", key)))?;

        match candidate.as_any_mut().downcast_mut::<DataWrapper<T>>() {
            Some(wrapper) => Ok(&mut wrapper.0),
            None => Err(SledError::new(format!(
                "Data with the key `{}` exists but it is not of type {}.",
                key,
                core::any::type_name::<T>()
            ))),
        }
    }

    pub fn set<T: StorableData>(&mut self, key: &str, value: T) {
        #[cfg(target_pointer_width = "64")]
        assert!(
            key.len() < 24,
            "Invalid data key; Max size is 24 bytes, `{}` is {} bytes.",
            key,
            key.len()
        );

        #[cfg(target_pointer_width = "32")]
        assert!(
            key.len() < 24,
            "Invalid data key; Max size is 12 bytes on 32-bit systems, `{}` is {} bytes.",
            key,
            key.len()
        );

        self.data.insert(
            key.to_compact_string(),
            Box::<DataWrapper<T>>::new(DataWrapper::new(value)),
        );
    }

    pub fn store<T: StorableData>(&mut self, key: &str, value: T) -> &mut T {
        #[cfg(target_pointer_width = "64")]
        assert!(
            key.len() < 24,
            "Invalid data key; Max size is 24 bytes, `{}` is {} bytes.",
            key,
            key.len()
        );

        #[cfg(target_pointer_width = "32")]
        assert!(
            key.len() < 24,
            "Invalid data key; Max size is 12 bytes on 32-bit systems, `{}` is {} bytes.",
            key,
            key.len()
        );

        self.data.insert(
            key.to_compact_string(),
            Box::<DataWrapper<T>>::new(DataWrapper::new(value)),
        );
        self.get_mut(key).unwrap()
    }

    pub fn empty_at(&self, key: &str) -> bool {
        !self.data.contains_key(key)
    }
}

impl Default for Data {
    fn default() -> Self {
        Self::new()
    }
}
