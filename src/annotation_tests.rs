/// Tests for annotation filtering feature
#[cfg(test)]
mod annotation_tests {
    use super::super::*;

    #[test]
    fn test_annotation_basic() {
        const SRC: &str = r#"
robot {
    sensor {
        lidar_range = 10m..50m
    }
}

minimal_demo {
    # @minimal
    # this is included because its marked minimal
    earth_sun = 1.5au
    
    # @minimal
    star_distance = 4.37light_years
    
}
"#;

        let eval = compile_code(SRC).expect("Failed to compile");

        // Check that annotation was detected
        assert!(
            eval.annotations.contains_key("minimal"),
            "Annotation 'minimal' not found"
        );

        // Get filtered string
        let filtered = eval
            .to_string_filtered("minimal")
            .expect("Failed to get filtered string");

        // Verify filtered string contains expected content
        assert!(
            filtered.contains("earth_sun"),
            "Filtered string missing earth_sun"
        );
        assert!(
            filtered.contains("star_distance"),
            "Filtered string missing star_distance"
        );
        assert!(
            filtered.contains("this is included"),
            "Filtered string missing comment"
        );

        // Should NOT contain robot config
        assert!(
            !filtered.contains("sensor"),
            "Filtered string should not contain unrelated config"
        );
    }

    #[test]
    fn test_annotation_with_namespaces() {
        const SRC: &str = r#"
sensor {
    lidar_range = 10m..50m
    # lidar comment
    camera_fov = 60
}

time {
    update_rate = 10ms
    # time update comment
    slow_rate = 1s
}

config {
    sensor {
        # @minimal
        # included sensor config
        lidar_range = 10m..50m
    }
    
    time {
        # @minimal
        # included time config
        update_rate = 10ms
    }
}
"#;

        let eval = compile_code(SRC).expect("Failed to compile");

        // Check annotation exists
        assert!(eval.annotations.contains_key("minimal"));

        // Get filtered string
        let filtered = eval
            .to_string_filtered("minimal")
            .expect("Failed to get filtered string");

        // Should contain annotated values with namespace context
        assert!(
            filtered.contains("lidar_range"),
            "Should contain annotated lidar_range"
        );
        assert!(
            filtered.contains("included sensor config"),
            "Should preserve comments"
        );
        assert!(
            filtered.contains("update_rate"),
            "Should contain annotated update_rate"
        );
        assert!(
            filtered.contains("included time config"),
            "Should preserve time comments"
        );

        // Should NOT contain non-annotated values
        assert!(
            !filtered.contains("camera_fov"),
            "Filtered string should not contain camera_fov"
        );
    }

    #[test]
    fn test_annotation_multiple_blocks() {
        const SRC: &str = r#"
# global config
timeout = 5000

# @minimal
max_distance = 1light_year

# @minimal
waypoint_precision = 1micron

# @advanced
max_retries = 100

# @advanced
retry_delay = 500ms

# more config
debug = true
"#;

        let eval = compile_code(SRC).expect("Failed to compile");

        // Check both annotations exist
        assert!(eval.annotations.contains_key("minimal"));
        assert!(eval.annotations.contains_key("advanced"));

        // Test minimal filter
        let minimal = eval
            .to_string_filtered("minimal")
            .expect("Failed to get minimal filter");
        assert!(minimal.contains("max_distance"));
        assert!(minimal.contains("waypoint_precision"));
        assert!(!minimal.contains("max_retries"));
        assert!(!minimal.contains("retry_delay"));

        // Test advanced filter
        let advanced = eval
            .to_string_filtered("advanced")
            .expect("Failed to get advanced filter");
        assert!(advanced.contains("max_retries"));
        assert!(advanced.contains("retry_delay"));
        assert!(!advanced.contains("max_distance"));
        assert!(!advanced.contains("waypoint_precision"));
    }

    #[test]
    fn test_annotation_preserves_comments() {
        const SRC: &str = r#"
config {
    # @filtered
    # comment 1
    value1 = 100
    
    # @filtered
    # comment 2
    value2 = 200
    
}
"#;

        let eval = compile_code(SRC).expect("Failed to compile");
        let filtered = eval
            .to_string_filtered("filtered")
            .expect("Failed to get filtered string");

        // Comments should be preserved
        assert!(filtered.contains("# comment 1"));
        assert!(filtered.contains("# comment 2"));

        // Values should be present
        assert!(filtered.contains("value1"));
        assert!(filtered.contains("value2"));
    }

    #[test]
    fn test_annotation_nonexistent_returns_none() {
        const SRC: &str = r#"
# @minimal
value = 100

"#;

        let eval = compile_code(SRC).expect("Failed to compile");

        // Request nonexistent annotation
        let filtered = eval.to_string_filtered("nonexistent");
        assert!(
            filtered.is_none(),
            "Should return None for nonexistent annotation"
        );
    }

    #[test]
    fn test_annotation_empty_block() {
        const SRC: &str = r#"
# @empty

other_value = 42
"#;

        let eval = compile_code(SRC).expect("Failed to compile");

        // Empty annotation should still be detected
        assert!(eval.annotations.contains_key("empty"));

        // Filtered output might be mostly empty/whitespace
        let filtered = eval.to_string_filtered("empty");
        assert!(
            filtered.is_some(),
            "Should return Some for empty annotation"
        );
    }

    #[test]
    fn test_annotation_with_block() {
        const SRC: &str = r#"
config {
    # @minimal
    sensor {
        lidar_range = 10m..50m
        camera_fov = 60
    }
    
    other = 200
}
"#;

        let eval = compile_code(SRC).expect("Failed to compile");

        // Annotation should exist
        assert!(eval.annotations.contains_key("minimal"));

        // Filtered should include entire block
        let minimal = eval
            .to_string_filtered("minimal")
            .expect("Failed to get minimal");
        assert!(minimal.contains("sensor"));
        assert!(minimal.contains("lidar_range"));
        assert!(minimal.contains("camera_fov"));
    }

    #[test]
    fn test_annotation_with_units() {
        const SRC: &str = r#"
# @minimal
# distance config
distance = 100mm..2m

# @minimal
# time config  
period = 30min

"#;

        let eval = compile_code(SRC).expect("Failed to compile");
        let filtered = eval
            .to_string_filtered("minimal")
            .expect("Failed to get filtered");

        // Should preserve unit values
        assert!(filtered.contains("100mm..2m"));
        assert!(filtered.contains("30min"));
        assert!(filtered.contains("distance config"));
        assert!(filtered.contains("time config"));
    }

    #[test]
    fn test_annotation_dotted_notation() {
        const SRC: &str = r#"
preprocessing {
    downsample {
        # @minimal
        # Output the full undistorted cloud
        bypass_output_downsample = true
    }
}

"#;

        let eval = compile_code(SRC).expect("Failed to compile");
        let filtered = eval
            .to_string_filtered("minimal")
            .expect("Failed to get filtered");

        // Should collapse to dotted notation at appropriate level
        assert!(filtered.contains("bypass_output_downsample"));
        assert!(filtered.contains("Output the full undistorted cloud"));
    }

    #[test]
    fn test_annotation_multiline_comments_in_block() {
        const SRC: &str = r#"
sensor {
    # @recommended
    # This is a multi-line comment.
    # It has several lines of explanation.
    # Line three of the comment.
    range = 10m..50m

    # @recommended
    # Another comment block.
    # Second line of second comment block.
    fov = 60
}
"#;

        let eval = compile_code(SRC).expect("Failed to compile");
        let filtered = eval
            .to_string_filtered("recommended")
            .expect("Failed to get filtered string");

        let expected = r#"sensor {
  # This is a multi-line comment.
  # It has several lines of explanation.
  # Line three of the comment.
  range = 10m..50m

  # Another comment block.
  # Second line of second comment block.
  fov = 60
}
"#;

        assert_eq!(filtered, expected);
    }

    #[test]
    fn test_annotation_multiline_comments_at_root() {
        const SRC: &str = r#"
# @recommended
# This is top-level comment line 1.
# This is top-level comment line 2.
timeout = 5000

# @recommended
# Another top-level comment line 1.
# Another top-level comment line 2.
retries = 3
"#;

        let eval = compile_code(SRC).expect("Failed to compile");
        let filtered = eval
            .to_string_filtered("recommended")
            .expect("Failed to get filtered string");

        let expected = r#"# This is top-level comment line 1.
# This is top-level comment line 2.
timeout = 5000

# Another top-level comment line 1.
# Another top-level comment line 2.
retries = 3
"#;

        assert_eq!(filtered, expected);
    }
}

