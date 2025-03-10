use std::fmt;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};


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
        NbtReprError::Structure(Box::new(error))
    }

    /// Creates a `NbtReprError` from the given error. If the given error is a [`NbtStructureError`],
    /// then the resulting representation error is of the `Structure` variant. If the error is a
    /// `NbtReprError` then it is downcasted and returned. All other error types are considered custom
    /// errors.
    pub fn from_any<E: Into<anyhow::Error>>(error: E) -> Self {
        let mut error = <E as Into<anyhow::Error>>::into(error);

        error = match error.downcast::<Self>() {
            Ok(error) => return error,
            Err(error) => error,
        };

        match error.downcast::<NbtStructureError>() {
            Ok(error) => NbtReprError::Structure(Box::new(error)),
            Err(error) => NbtReprError::Custom(error),
        }
    }
}

impl From<NbtStructureError> for NbtReprError {
    fn from(error: NbtStructureError) -> Self {
        NbtReprError::Structure(Box::new(error))
    }
}

impl From<Box<NbtStructureError>> for NbtReprError {
    fn from(error: Box<NbtStructureError>) -> Self {
        NbtReprError::Structure(error)
    }
}

impl Display for NbtReprError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            NbtReprError::Structure(error) => Display::fmt(error, f),
            NbtReprError::Custom(custom) => Display::fmt(custom, f),
        }
    }
}

impl Error for NbtReprError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            NbtReprError::Structure(error) => Some(error),
            NbtReprError::Custom(custom) => Some(&**custom),
        }
    }
}

/// An error associated with the structure of an NBT tag tree. This error represents a conflict
/// between the expected and actual structure of an NBT tag tree.
#[repr(transparent)]
pub struct NbtStructureError {
    repr: NbtStructureErrorRepr,
}

impl NbtStructureError {
    pub(crate) fn missing_tag<T: Into<String>>(tag_name: T) -> Self {
        NbtStructureError {
            repr: NbtStructureErrorRepr::MissingTag {
                tag_name: tag_name.into().into_boxed_str(),
            },
        }
    }

    pub(crate) fn invalid_index(index: usize, length: usize) -> Self {
        NbtStructureError {
            repr: NbtStructureErrorRepr::InvalidIndex { index, length },
        }
    }

    pub(crate) fn type_mismatch(expected: &'static str, found: &'static str) -> Self {
        NbtStructureError {
            repr: NbtStructureErrorRepr::TypeMismatch { expected, found },
        }
    }
}

impl Debug for NbtStructureError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.repr, f)
    }
}

impl Display for NbtStructureError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.repr {
            NbtStructureErrorRepr::MissingTag { tag_name } =>
                write!(f, "Missing tag \"{}\"", tag_name),
            NbtStructureErrorRepr::InvalidIndex { index, length } =>
                write!(f, "Index out of range: {} >= {}", index, length),
            NbtStructureErrorRepr::TypeMismatch { expected, found } => write!(
                f,
                "Tag type mismatch: expected {} but found {}",
                expected, found
            ),
        }
    }
}

impl Error for NbtStructureError {}

#[derive(Debug)]
enum NbtStructureErrorRepr {
    MissingTag {
        tag_name: Box<str>,
    },
    InvalidIndex {
        index: usize,
        length: usize,
    },
    TypeMismatch {
        expected: &'static str,
        found: &'static str,
    },
}
