use std::borrow::Cow;

#[allow(dead_code)]
pub(crate) fn join_path(
    base_path: &str,
    path_suffix: &str,
    path_params: Vec<Cow<'static, str>>,
) -> String {
    let base_path_segments: Vec<&str> = base_path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    let path_suffix_segments: Vec<&str> = path_suffix
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();

    let segments: Vec<&str> = Vec::from_iter(
        base_path_segments
            .into_iter()
            .chain(path_params.iter().map(|param| param.as_ref()))
            .chain(path_suffix_segments.into_iter()),
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
