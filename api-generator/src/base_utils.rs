use std::borrow::Cow;

#[allow(dead_code)]
pub(crate) fn join_path(base_path: &str, path_suffix: &str, path_params: Vec<Cow<'static, str>>) -> String {
    let base_path_segments = base_path.split('/').filter(|seg| !seg.is_empty());
    let path_suffix_segments = path_suffix.split('/').filter(|seg| !seg.is_empty());

    let segments: Vec<&str> = Vec::from_iter(
        base_path_segments
            .chain(path_params.iter().map(|param| param.as_ref()))
            .chain(path_suffix_segments),
    );
    segments.join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_join_path() {
        assert_eq!(
            "prefix/prefix2/prefix3/abc/123/def/suffix/suffix2/suffix3",
            join_path(
                "/prefix/prefix2/prefix3",
                "/suffix/suffix2/suffix3",
                vec!["abc".into(), "123".into(), "def".into()]
            )
        );
    }
}
