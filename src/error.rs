use thiserror::Error;

/// All errors that openmem can produce.
#[derive(Error, Debug)]
pub enum OpenMemError {
    #[error("Node not found: {0}")]
    NodeNotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("VCS error: {0}")]
    VcsError(String),
}

pub type Result<T> = std::result::Result<T, OpenMemError>;

#[cfg(test)]
mod tests {
    use super::*;

    // --- Positive tests ---

    #[test]
    fn error_displays_node_not_found() {
        let err = OpenMemError::NodeNotFound("global/user-prefs".into());
        assert_eq!(err.to_string(), "Node not found: global/user-prefs");
    }

    #[test]
    fn error_displays_vcs_error() {
        let err = OpenMemError::VcsError("jj not found".into());
        assert_eq!(err.to_string(), "VCS error: jj not found");
    }

    #[test]
    fn error_converts_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file gone");
        let err: OpenMemError = io_err.into();
        assert!(matches!(err, OpenMemError::Io(_)));
        assert!(err.to_string().contains("file gone"));
    }

    // --- Negative tests: ensure error identity ---

    #[test]
    fn error_node_not_found_preserves_path() {
        let path = "projects/acme/frontend";
        let err = OpenMemError::NodeNotFound(path.into());
        // The exact path must be preserved in the error
        assert!(err.to_string().contains(path));
    }

    #[test]
    fn error_variants_are_distinct() {
        let e1 = OpenMemError::NodeNotFound("x".into());
        let e2 = OpenMemError::VcsError("x".into());
        let e3: OpenMemError = std::io::Error::new(std::io::ErrorKind::Other, "x").into();

        // Each variant matches only itself
        assert!(matches!(e1, OpenMemError::NodeNotFound(_)));
        assert!(!matches!(e1, OpenMemError::Io(_)));
        assert!(matches!(e2, OpenMemError::VcsError(_)));
        assert!(!matches!(e2, OpenMemError::NodeNotFound(_)));
        assert!(matches!(e3, OpenMemError::Io(_)));
        assert!(!matches!(e3, OpenMemError::VcsError(_)));
    }

    #[test]
    fn result_type_works_with_question_mark() {
        fn might_fail(should_fail: bool) -> Result<String> {
            if should_fail {
                Err(OpenMemError::NodeNotFound("test".into()))
            } else {
                Ok("success".into())
            }
        }

        assert!(might_fail(false).is_ok());
        assert!(might_fail(true).is_err());
    }
}
