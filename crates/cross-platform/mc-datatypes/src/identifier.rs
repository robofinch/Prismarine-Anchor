use std::{fmt, mem};
use std::fmt::{Display, Formatter};

use thiserror::Error;


/// Namespaced identifiers are also known as resource locations.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NamespacedIdentifier {
    pub namespace: Box<str>,
    pub path:      Box<str>,
}

impl NamespacedIdentifier {
    pub fn parse_string(
        mut identifier: String,
        opts:           IdentifierParseOptions,
    ) -> Result<Self, IdentifierParseError> {
        let path = match identifier.find(':') {
            // "+ 1" because the UTF-8 byte length of ':' is 1
            Some(colon_pos) => {
                let path = identifier.split_off(colon_pos + 1);
                // Pop the colon
                identifier.pop();
                path
            }
            None => {
                if let Some(namespace) = opts.default_namespace {
                    mem::replace(&mut identifier, namespace.to_owned())
                } else {
                    let mut quoted = String::with_capacity(identifier.len() + 2);
                    quoted.push('\"');
                    quoted.push_str(&identifier);
                    quoted.push('\"');
                    return Err(IdentifierParseError::InvalidIdentifier(quoted));
                }
            }
        };

        let namespace = identifier;

        // Validate the namespace and path
        if opts.java_character_constraints {
            // If we can find a character which is not allowed, return an error.
            if let Some(ch) = namespace.chars().find(|&ch| {
                let allowed = ch.is_ascii_digit()
                    || ch.is_ascii_lowercase()
                    || ['_', '-', '.'].contains(&ch);
                !allowed
            }) {
                return Err(IdentifierParseError::InvalidNamespaceCharacter(path, ch));
            }

            if let Some(ch) = path.chars().find(|&ch| {
                let allowed = ch.is_ascii_digit()
                    || ch.is_ascii_lowercase()
                    || ['_', '-', '.', '/'].contains(&ch);
                !allowed
            }) {
                return Err(IdentifierParseError::InvalidPathCharacter(path, ch));
            }
        } else {
            // The character constraints used by Bedrock are a lot looser
            if namespace.find(':').is_some() {
                return Err(IdentifierParseError::InvalidNamespaceCharacter(path, ':'));
            }
            if namespace.find('/').is_some() {
                return Err(IdentifierParseError::InvalidNamespaceCharacter(path, '/'));
            }
            if path.find(':').is_some() {
                return Err(IdentifierParseError::InvalidPathCharacter(path, ':'));
            }
        }

        Ok(Self {
            namespace: namespace.into_boxed_str(),
            path:      path.into_boxed_str(),
        })
    }
}

impl Display for NamespacedIdentifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.namespace, self.path)
    }
}

/// Parse options for [`NamespacedIdentifier`]s, also known as Resource Locations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdentifierParseOptions {
    /// If `Some`, if the `namespace:` part of `namespace:path` is missing, assume
    /// that the namespace is this string. If this is `None` and a namespace is missing,
    /// an error is returned from appropriate functions.
    pub default_namespace:          Option<&'static str>,
    /// If true, use Java Edition's stricter restrictions for the characters
    /// which may appear in a [`NamespacedIdentifier`].
    pub java_character_constraints: bool,
}

impl Default for IdentifierParseOptions {
    /// Defaults to the strictest settings.
    #[inline]
    fn default() -> Self {
        Self {
            default_namespace:          None,
            java_character_constraints: true,
        }
    }
}

#[derive(Error, Debug, Clone)]
pub enum IdentifierParseError {
    #[error("expected a string identifier in the form \"namespace:path\", but receieved {0}")]
    InvalidIdentifier(String),
    #[error("invalid character '{1}' in the namespace of \"{0}\"")]
    InvalidNamespaceCharacter(String, char),
    #[error("invalid character '{1}' in the path of \"{0}\"")]
    InvalidPathCharacter(String, char),
}
