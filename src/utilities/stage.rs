use std::collections::HashSet;

/// Resolves prerequisites for a stage, detecting circular dependencies and deduplicating
/// Returns the stages to execute in order, or an error if circular dependencies are detected
pub fn resolve_prerequisites(
    stage_name: &str,
    prerequisites: &[String],
    get_prereqs: &dyn Fn(&str) -> Option<Vec<String>>,
) -> Result<Vec<String>, String> {
    let mut result = Vec::new();
    let mut visited = HashSet::new();
    let mut in_progress = HashSet::new();

    resolve_prerequisites_recursive(
        stage_name,
        prerequisites,
        get_prereqs,
        &mut result,
        &mut visited,
        &mut in_progress,
    )?;

    Ok(result)
}

/// Recursive helper for prerequisite resolution with cycle detection
fn resolve_prerequisites_recursive(
    current_stage: &str,
    prerequisites: &[String],
    get_prereqs: &dyn Fn(&str) -> Option<Vec<String>>,
    result: &mut Vec<String>,
    visited: &mut HashSet<String>,
    in_progress: &mut HashSet<String>,
) -> Result<(), String> {
    // Mark current stage as in progress (for cycle detection)
    in_progress.insert(current_stage.to_string());

    for prereq in prerequisites {
        // Check for circular dependency
        if in_progress.contains(prereq) {
            let chain: Vec<_> = in_progress.iter().collect();
            return Err(format!(
                "Circular dependency detected: {} is already in the prerequisite chain ({:?})",
                prereq, chain
            ));
        }

        // Skip if already visited
        if visited.contains(prereq) {
            continue;
        }

        // Get prerequisites for this prerequisite
        if let Some(nested_prereqs) = get_prereqs(prereq) {
            resolve_prerequisites_recursive(
                prereq,
                &nested_prereqs,
                get_prereqs,
                result,
                visited,
                in_progress,
            )?;
        }

        // Add to result and mark as visited
        if !visited.contains(prereq) {
            result.push(prereq.clone());
            visited.insert(prereq.clone());
        }
    }

    // Remove from in_progress once done processing this stage
    in_progress.remove(current_stage);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_resolve_prerequisites_simple() {
        let prereq_map: HashMap<&str, Vec<String>> = HashMap::new();
        let get_prereqs = |stage: &str| prereq_map.get(stage).cloned();

        let result = resolve_prerequisites(
            "install",
            &["build".to_string()],
            &get_prereqs,
        );

        assert!(result.is_ok());
        let stages = result.unwrap();
        assert_eq!(stages, vec!["build"]);
    }

    #[test]
    fn test_resolve_prerequisites_chain() {
        let mut prereq_map: HashMap<&str, Vec<String>> = HashMap::new();
        prereq_map.insert("stage2", vec!["stage1".to_string()]);
        let get_prereqs = |stage: &str| prereq_map.get(stage).cloned();

        let result = resolve_prerequisites(
            "stage3",
            &["stage2".to_string()],
            &get_prereqs,
        );

        assert!(result.is_ok());
        let stages = result.unwrap();
        assert_eq!(stages, vec!["stage1", "stage2"]);
    }

    #[test]
    fn test_resolve_prerequisites_deduplicate() {
        let prereq_map: HashMap<&str, Vec<String>> = HashMap::new();
        let get_prereqs = |stage: &str| prereq_map.get(stage).cloned();

        let result = resolve_prerequisites(
            "install",
            &["build".to_string(), "build".to_string()],
            &get_prereqs,
        );

        assert!(result.is_ok());
        let stages = result.unwrap();
        assert_eq!(stages, vec!["build"]); // Should only appear once
    }

    #[test]
    fn test_resolve_prerequisites_multiple() {
        let mut prereq_map: HashMap<&str, Vec<String>> = HashMap::new();
        prereq_map.insert("build", vec!["clean".to_string()]);
        let get_prereqs = |stage: &str| prereq_map.get(stage).cloned();

        let result = resolve_prerequisites(
            "install",
            &["build".to_string(), "test".to_string()],
            &get_prereqs,
        );

        assert!(result.is_ok());
        let stages = result.unwrap();
        // Should have clean (from build's prereqs), then build, then test
        assert_eq!(stages, vec!["clean", "build", "test"]);
    }

    #[test]
    fn test_resolve_prerequisites_circular() {
        let mut prereq_map: HashMap<&str, Vec<String>> = HashMap::new();
        prereq_map.insert("stage1", vec!["stage2".to_string()]);
        prereq_map.insert("stage2", vec!["stage1".to_string()]);
        let get_prereqs = |stage: &str| prereq_map.get(stage).cloned();

        let result = resolve_prerequisites(
            "start",
            &["stage1".to_string()],
            &get_prereqs,
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Circular dependency detected"));
    }

    #[test]
    fn test_resolve_prerequisites_self_reference() {
        let mut prereq_map: HashMap<&str, Vec<String>> = HashMap::new();
        prereq_map.insert("stage1", vec!["stage1".to_string()]);
        let get_prereqs = |stage: &str| prereq_map.get(stage).cloned();

        let result = resolve_prerequisites(
            "start",
            &["stage1".to_string()],
            &get_prereqs,
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Circular dependency detected"));
    }

    #[test]
    fn test_resolve_prerequisites_complex_chain() {
        let mut prereq_map: HashMap<&str, Vec<String>> = HashMap::new();
        prereq_map.insert("uninstall", vec!["backup".to_string()]);
        prereq_map.insert("build", vec!["clean".to_string()]);
        prereq_map.insert("install", vec!["build".to_string()]);
        let get_prereqs = |stage: &str| prereq_map.get(stage).cloned();

        // Update has prerequisites: uninstall, build, install
        let result = resolve_prerequisites(
            "update",
            &["uninstall".to_string(), "build".to_string(), "install".to_string()],
            &get_prereqs,
        );

        assert!(result.is_ok());
        let stages = result.unwrap();
        // Should be: backup, uninstall, clean, build, install
        assert_eq!(stages, vec!["backup", "uninstall", "clean", "build", "install"]);
    }

    #[test]
    fn test_resolve_prerequisites_diamond() {
        let mut prereq_map: HashMap<&str, Vec<String>> = HashMap::new();
        // Diamond dependency: both stage2 and stage3 depend on stage1
        prereq_map.insert("stage2", vec!["stage1".to_string()]);
        prereq_map.insert("stage3", vec!["stage1".to_string()]);
        let get_prereqs = |stage: &str| prereq_map.get(stage).cloned();

        let result = resolve_prerequisites(
            "stage4",
            &["stage2".to_string(), "stage3".to_string()],
            &get_prereqs,
        );

        assert!(result.is_ok());
        let stages = result.unwrap();
        // stage1 should only appear once, even though both stage2 and stage3 depend on it
        assert_eq!(stages, vec!["stage1", "stage2", "stage3"]);
    }
}

