use std::{error, fmt};
use std::fmt::{Debug, Display, Formatter};

use thiserror::Error;


#[derive(Debug)]
pub enum NbtReprError {
    /// A structure error in the tag tree.
    Structure(Box<NbtStructureError>),
    /// A custom error.
    Custom(anyhow::Error),
}

impl NbtReprError {
    /// Creates a new NBT representation error from the given structure error.
    pub fn structure(error: NbtStructureError) -> Self {
        Self::Structure(Box::new(error))
    }

    /// Creates a `NbtReprError` from the given error. If the given error is a
    /// [`NbtStructureError`], then the resulting representation error is of the `Structure`
    /// variant. If the error is a `NbtReprError` then it is downcasted and returned. All other
    /// error types are considered custom errors.
    pub fn from_any<E: Into<anyhow::Error>>(error: E) -> Self {
        let mut error = <E as Into<anyhow::Error>>::into(error);

        error = match error.downcast::<Self>() {
            Ok(error)  => return error,
            Err(error) => error,
        };

        match error.downcast::<NbtStructureError>() {
            Ok(error)  => Self::Structure(Box::new(error)),
            Err(error) => Self::Custom(error),
        }
    }
}

impl From<NbtStructureError> for NbtReprError {
    fn from(error: NbtStructureError) -> Self {
        Self::Structure(Box::new(error))
    }
}

impl From<Box<NbtStructureError>> for NbtReprError {
    fn from(error: Box<NbtStructureError>) -> Self {
        Self::Structure(error)
    }
}

impl Display for NbtReprError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Structure(error) => Display::fmt(error, f),
            Self::Custom(custom)   => Display::fmt(custom, f),
        }
    }
}

impl error::Error for NbtReprError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::Structure(error) => Some(error),
            Self::Custom(custom)   => Some(&**custom),
        }
    }
}

/// An error associated with the structure of an NBT tag tree. This error represents a conflict
/// between the expected and actual structure of an NBT tag tree.
#[derive(Error, Debug, Clone)]
pub enum NbtStructureError {
    #[error("Missing tag \"{tag_name}\"")]
    MissingTag {
        tag_name: Box<str>,
    },
    #[error("Index out of range: {index} >= {length}")]
    InvalidIndex {
        index:  usize,
        length: usize,
    },
    #[error("Tag type mismatch: expected {expected} but found {found}")]
    TypeMismatch {
        expected: &'static str,
        found:    &'static str,
    },
}

impl NbtStructureError {
    pub fn missing_tag<T: Into<String>>(tag_name: T) -> Self {
        Self::MissingTag {
            tag_name: tag_name.into().into_boxed_str(),
        }
    }

    pub fn invalid_index(index: usize, length: usize) -> Self {
        Self::InvalidIndex { index, length }
    }

    pub fn type_mismatch(expected: &'static str, found: &'static str) -> Self {
        Self::TypeMismatch { expected, found }
    }
}
