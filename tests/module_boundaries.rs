#[path = "module_boundaries/abi_decision_state_is_policy_only.rs"]
mod abi_decision_state_is_policy_only;
#[path = "module_boundaries/abi_module_owns_abi_surface_policy.rs"]
mod abi_module_owns_abi_surface_policy;
#[path = "module_boundaries/abi_policy_config_is_profile_abi_policy_model.rs"]
mod abi_policy_config_is_profile_abi_policy_model;
#[path = "module_boundaries/arch_policy_config_is_profile_arch_policy_model.rs"]
mod arch_policy_config_is_profile_arch_policy_model;
#[path = "module_boundaries/architecture_doc_describes_module_ownership_and_dependencies.rs"]
mod architecture_doc_describes_module_ownership_and_dependencies;
#[path = "module_boundaries/authoritative_lockfile_update_is_final_publication_step.rs"]
mod authoritative_lockfile_update_is_final_publication_step;
#[path = "module_boundaries/build_matrix_config_is_profile_build_matrix_model.rs"]
mod build_matrix_config_is_profile_build_matrix_model;
#[path = "module_boundaries/candidate_build_does_not_open_output_repo_before_commit.rs"]
mod candidate_build_does_not_open_output_repo_before_commit;
#[path = "module_boundaries/candidate_state_cannot_update_lockfile_apis_directly.rs"]
mod candidate_state_cannot_update_lockfile_apis_directly;
#[path = "module_boundaries/candidate_tree_state_is_candidate_only.rs"]
mod candidate_tree_state_is_candidate_only;
#[path = "module_boundaries/candidate_verification_is_verification_proof.rs"]
mod candidate_verification_is_verification_proof;
#[path = "module_boundaries/ci_runs_rust_source_size_policy_check.rs"]
mod ci_runs_rust_source_size_policy_check;
#[path = "module_boundaries/cli_module_owns_command_line_shape_commands_module_owns_dispatch.rs"]
mod cli_module_owns_command_line_shape_commands_module_owns_dispatch;
#[path = "module_boundaries/command_execution_delegates_reducer_semantics.rs"]
mod command_execution_delegates_reducer_semantics;
#[path = "module_boundaries/config_tests_are_behavior_focused.rs"]
mod config_tests_are_behavior_focused;
#[path = "module_boundaries/core_module_owns_shared_foundations.rs"]
mod core_module_owns_shared_foundations;
#[path = "module_boundaries/common.rs"]
mod common;
#[path = "module_boundaries/cpp_folding_module_removes_only_proven_dead_branches.rs"]
mod cpp_folding_module_removes_only_proven_dead_branches;
#[path = "module_boundaries/device_binding_removal_requires_no_live_dts_or_schema_reference_proof.rs"]
mod device_binding_removal_requires_no_live_dts_or_schema_reference_proof;
#[path = "module_boundaries/diagnostics_command_capture_lives_in_command_capture_module.rs"]
mod diagnostics_command_capture_lives_in_command_capture_module;
#[path = "module_boundaries/diagnostics_classifier_lives_in_classifier_module.rs"]
mod diagnostics_classifier_lives_in_classifier_module;
#[path = "module_boundaries/diagnostics_model_lives_in_model_module.rs"]
mod diagnostics_model_lives_in_model_module;
#[path = "module_boundaries/diagnostics_renderer_lives_in_renderer_module.rs"]
mod diagnostics_renderer_lives_in_renderer_module;
#[path = "module_boundaries/diagnostics_unit_tests_live_beside_owned_modules.rs"]
mod diagnostics_unit_tests_live_beside_owned_modules;
#[path = "module_boundaries/documentation_appendices_live_in_reference_docs.rs"]
mod documentation_appendices_live_in_reference_docs;
#[path = "module_boundaries/edit_reason_model_lives_in_reason_module.rs"]
mod edit_reason_model_lives_in_reason_module;
#[path = "module_boundaries/edit_proof_source_model_lives_in_proof_source_module.rs"]
mod edit_proof_source_model_lives_in_proof_source_module;
#[path = "module_boundaries/edit_reason_serialization_lives_in_serialization_module.rs"]
mod edit_reason_serialization_lives_in_serialization_module;
#[path = "module_boundaries/edit_validation_lives_in_validation_module.rs"]
mod edit_validation_lives_in_validation_module;
#[path = "module_boundaries/edit_reason_render_helpers_live_in_render_module.rs"]
mod edit_reason_render_helpers_live_in_render_module;
#[path = "module_boundaries/exported_symbol_provider_removal_requires_no_live_consumer_proof.rs"]
mod exported_symbol_provider_removal_requires_no_live_consumer_proof;
#[path = "module_boundaries/execution_module_owns_process_execution_boundaries.rs"]
mod execution_module_owns_process_execution_boundaries;
#[path = "module_boundaries/failure_state_cannot_convert_into_published_state.rs"]
mod failure_state_cannot_convert_into_published_state;
#[path = "module_boundaries/feature_config_is_named_feature_intent_model.rs"]
mod feature_config_is_named_feature_intent_model;
#[path = "module_boundaries/feature_conflict_detection_reports_live_device_id_table_references.rs"]
mod feature_conflict_detection_reports_live_device_id_table_references;
#[path = "module_boundaries/feature_conflict_detection_reports_live_exported_symbol_consumers.rs"]
mod feature_conflict_detection_reports_live_exported_symbol_consumers;
#[path = "module_boundaries/feature_conflict_detection_reports_live_kbuild_references.rs"]
mod feature_conflict_detection_reports_live_kbuild_references;
#[path = "module_boundaries/feature_conflict_detection_reports_live_kconfig_selects.rs"]
mod feature_conflict_detection_reports_live_kconfig_selects;
#[path = "module_boundaries/feature_conflict_detection_reports_live_userspace_uapi_references.rs"]
mod feature_conflict_detection_reports_live_userspace_uapi_references;
#[path = "module_boundaries/feature_conflict_detection_reports_reachable_runtime_registrations.rs"]
mod feature_conflict_detection_reports_reachable_runtime_registrations;
#[path = "module_boundaries/feature_conflict_detection_reports_removed_live_dependency_edges.rs"]
mod feature_conflict_detection_reports_removed_live_dependency_edges;
#[path = "module_boundaries/feature_conflict_detection_reports_shared_removed_preserved_files.rs"]
mod feature_conflict_detection_reports_shared_removed_preserved_files;
#[path = "module_boundaries/feature_conflict_report_is_semantic_conflict_report_model.rs"]
mod feature_conflict_report_is_semantic_conflict_report_model;
#[path = "module_boundaries/feature_conflicts_are_rendered_as_actionable_output.rs"]
mod feature_conflicts_are_rendered_as_actionable_output;
#[path = "module_boundaries/feature_edge_is_semantic_feature_edge_model.rs"]
mod feature_edge_is_semantic_feature_edge_model;
#[path = "module_boundaries/feature_graph_is_semantic_feature_graph_model.rs"]
mod feature_graph_is_semantic_feature_graph_model;
#[path = "module_boundaries/feature_id_is_stable_feature_identity_model.rs"]
mod feature_id_is_stable_feature_identity_model;
#[path = "module_boundaries/feature_impact_report_is_semantic_impact_report_model.rs"]
mod feature_impact_report_is_semantic_impact_report_model;
#[path = "module_boundaries/feature_intent_is_semantic_intent_model.rs"]
mod feature_intent_is_semantic_intent_model;
#[path = "module_boundaries/feature_kind_is_stable_feature_kind_model.rs"]
mod feature_kind_is_stable_feature_kind_model;
#[path = "module_boundaries/feature_module_owns_feature_graph_resolution_and_conflicts.rs"]
mod feature_module_owns_feature_graph_resolution_and_conflicts;
#[path = "module_boundaries/feature_node_is_semantic_feature_node_model.rs"]
mod feature_node_is_semantic_feature_node_model;
#[path = "module_boundaries/feature_ownership_is_semantic_ownership_model.rs"]
mod feature_ownership_is_semantic_ownership_model;
#[path = "module_boundaries/feature_resolution_state_is_resolution_only.rs"]
mod feature_resolution_state_is_resolution_only;
#[path = "module_boundaries/feature_root_is_typed_feature_root_model.rs"]
mod feature_root_is_typed_feature_root_model;
#[path = "module_boundaries/feature_roots_resolve_to_acpi_ids.rs"]
mod feature_roots_resolve_to_acpi_ids;
#[path = "module_boundaries/feature_roots_resolve_to_devicetree_compatibles.rs"]
mod feature_roots_resolve_to_devicetree_compatibles;
#[path = "module_boundaries/feature_roots_resolve_to_docs.rs"]
mod feature_roots_resolve_to_docs;
#[path = "module_boundaries/feature_roots_resolve_to_exported_symbols.rs"]
mod feature_roots_resolve_to_exported_symbols;
#[path = "module_boundaries/feature_roots_resolve_to_firmware_paths.rs"]
mod feature_roots_resolve_to_firmware_paths;
#[path = "module_boundaries/feature_roots_resolve_to_generated_artifacts.rs"]
mod feature_roots_resolve_to_generated_artifacts;
#[path = "module_boundaries/feature_roots_resolve_to_initcalls.rs"]
mod feature_roots_resolve_to_initcalls;
#[path = "module_boundaries/feature_roots_resolve_to_kbuild_objects.rs"]
mod feature_roots_resolve_to_kbuild_objects;
#[path = "module_boundaries/feature_roots_resolve_to_kconfig_symbols.rs"]
mod feature_roots_resolve_to_kconfig_symbols;
#[path = "module_boundaries/feature_roots_resolve_to_kselftest_targets.rs"]
mod feature_roots_resolve_to_kselftest_targets;
#[path = "module_boundaries/feature_roots_resolve_to_kunit_suites.rs"]
mod feature_roots_resolve_to_kunit_suites;
#[path = "module_boundaries/feature_roots_resolve_to_module_aliases.rs"]
mod feature_roots_resolve_to_module_aliases;
#[path = "module_boundaries/feature_roots_resolve_to_module_names.rs"]
mod feature_roots_resolve_to_module_names;
#[path = "module_boundaries/feature_roots_resolve_to_paths.rs"]
mod feature_roots_resolve_to_paths;
#[path = "module_boundaries/feature_roots_resolve_to_pci_ids.rs"]
mod feature_roots_resolve_to_pci_ids;
#[path = "module_boundaries/feature_roots_resolve_to_private_headers.rs"]
mod feature_roots_resolve_to_private_headers;
#[path = "module_boundaries/feature_roots_resolve_to_public_headers.rs"]
mod feature_roots_resolve_to_public_headers;
#[path = "module_boundaries/feature_roots_resolve_to_runtime_registrations.rs"]
mod feature_roots_resolve_to_runtime_registrations;
#[path = "module_boundaries/feature_roots_resolve_to_samples.rs"]
mod feature_roots_resolve_to_samples;
#[path = "module_boundaries/feature_roots_resolve_to_source_files.rs"]
mod feature_roots_resolve_to_source_files;
#[path = "module_boundaries/feature_roots_resolve_to_tools.rs"]
mod feature_roots_resolve_to_tools;
#[path = "module_boundaries/feature_roots_resolve_to_uapi_headers.rs"]
mod feature_roots_resolve_to_uapi_headers;
#[path = "module_boundaries/feature_roots_resolve_to_usb_ids.rs"]
mod feature_roots_resolve_to_usb_ids;
#[path = "module_boundaries/feature_scope_is_typed_feature_scope_model.rs"]
mod feature_scope_is_typed_feature_scope_model;
#[path = "module_boundaries/file_size_policy_doc_describes_soft_cap_and_split_rules.rs"]
mod file_size_policy_doc_describes_soft_cap_and_split_rules;
#[path = "module_boundaries/filesystem_mutation_is_separate_from_proof_generation.rs"]
mod filesystem_mutation_is_separate_from_proof_generation;
#[path = "module_boundaries/fixup_application_lives_in_application_module.rs"]
mod fixup_application_lives_in_application_module;
#[path = "module_boundaries/fixup_diagnostic_classification_lives_in_classification_module.rs"]
mod fixup_diagnostic_classification_lives_in_classification_module;
#[path = "module_boundaries/fixup_planning_lives_in_planning_module.rs"]
mod fixup_planning_lives_in_planning_module;
#[path = "module_boundaries/fixup_reindexing_lives_in_reindex_module.rs"]
mod fixup_reindexing_lives_in_reindex_module;
#[path = "module_boundaries/fixup_report_types_live_in_report_module.rs"]
mod fixup_report_types_live_in_report_module;
#[path = "module_boundaries/fixups_mutate_only_with_diagnostic_manifest_and_index_proof.rs"]
mod fixups_mutate_only_with_diagnostic_manifest_and_index_proof;
#[path = "module_boundaries/generate_attempt_failure_is_attempt_only.rs"]
mod generate_attempt_failure_is_attempt_only;
#[path = "module_boundaries/generated_module_owns_artifact_policy_and_clean_build_verification.rs"]
mod generated_module_owns_artifact_policy_and_clean_build_verification;
#[path = "module_boundaries/generate_candidate_materialization_glue_lives_in_candidate_module.rs"]
mod generate_candidate_materialization_glue_lives_in_candidate_module;
#[path = "module_boundaries/generate_candidate_modules_define_candidate_build_boundaries.rs"]
mod generate_candidate_modules_define_candidate_build_boundaries;
#[path = "module_boundaries/generate_failure_reporting_and_rollback_glue_lives_in_failure_module.rs"]
mod generate_failure_reporting_and_rollback_glue_lives_in_failure_module;
#[path = "module_boundaries/generate_options_live_in_options_module.rs"]
mod generate_options_live_in_options_module;
#[path = "module_boundaries/generate_orchestration_entrypoint_lives_in_orchestration_module.rs"]
mod generate_orchestration_entrypoint_lives_in_orchestration_module;
#[path = "module_boundaries/generate_plan_report_lives_in_plan_report_module.rs"]
mod generate_plan_report_lives_in_plan_report_module;
#[path = "module_boundaries/generate_plan_is_immutable_candidate_truth_object.rs"]
mod generate_plan_is_immutable_candidate_truth_object;
#[path = "module_boundaries/generate_state_tests_are_behavior_focused.rs"]
mod generate_state_tests_are_behavior_focused;
#[path = "module_boundaries/generate_publish_module_defines_publication_boundaries.rs"]
mod generate_publish_module_defines_publication_boundaries;
#[path = "module_boundaries/generate_verify_modules_define_candidate_verification_boundaries.rs"]
mod generate_verify_modules_define_candidate_verification_boundaries;
#[path = "module_boundaries/generate_verification_glue_lives_in_verify_module.rs"]
mod generate_verification_glue_lives_in_verify_module;
#[path = "module_boundaries/hardware_module_owns_device_matching_and_binding_proof.rs"]
mod hardware_module_owns_device_matching_and_binding_proof;
#[path = "module_boundaries/include_cleanup_module_removes_only_proven_include_sites.rs"]
mod include_cleanup_module_removes_only_proven_include_sites;
#[path = "module_boundaries/include_cleanup_rewrite_lives_in_cleanup_module.rs"]
mod include_cleanup_rewrite_lives_in_cleanup_module;
#[path = "module_boundaries/include_index_lives_in_index_module.rs"]
mod include_index_lives_in_index_module;
#[path = "module_boundaries/include_private_header_orphaning_lives_in_private_header_module.rs"]
mod include_private_header_orphaning_lives_in_private_header_module;
#[path = "module_boundaries/include_public_header_policy_lives_in_policy_module.rs"]
mod include_public_header_policy_lives_in_policy_module;
#[path = "module_boundaries/include_unit_tests_live_beside_owned_modules.rs"]
mod include_unit_tests_live_beside_owned_modules;
#[path = "module_boundaries/integration_tests_are_split_by_feature_area.rs"]
mod integration_tests_are_split_by_feature_area;
#[path = "module_boundaries/kernel_indexes_rebuild_only_through_controlled_api.rs"]
mod kernel_indexes_rebuild_only_through_controlled_api;
#[path = "module_boundaries/kbuild_ast_lives_in_ast_module.rs"]
mod kbuild_ast_lives_in_ast_module;
#[path = "module_boundaries/kbuild_object_graph_lives_in_object_graph_module.rs"]
mod kbuild_object_graph_lives_in_object_graph_module;
#[path = "module_boundaries/kbuild_parser_lives_in_parser_module.rs"]
mod kbuild_parser_lives_in_parser_module;
#[path = "module_boundaries/kbuild_report_types_live_in_report_module.rs"]
mod kbuild_report_types_live_in_report_module;
#[path = "module_boundaries/kbuild_rewrite_module_preserves_unrelated_makefile_content.rs"]
mod kbuild_rewrite_module_preserves_unrelated_makefile_content;
#[path = "module_boundaries/kbuild_rewrite_lives_in_rewrite_module.rs"]
mod kbuild_rewrite_lives_in_rewrite_module;
#[path = "module_boundaries/kbuild_unit_tests_live_beside_owned_modules.rs"]
mod kbuild_unit_tests_live_beside_owned_modules;
#[path = "module_boundaries/legacy_catch_all_files_do_not_accumulate_new_logic.rs"]
mod legacy_catch_all_files_do_not_accumulate_new_logic;
#[path = "module_boundaries/no_new_top_level_feature_modules_after_subsystem_dir_exists.rs"]
mod no_new_top_level_feature_modules_after_subsystem_dir_exists;
#[path = "module_boundaries/no_duplicate_module_roots_without_facade_or_legacy_allowlist.rs"]
mod no_duplicate_module_roots_without_facade_or_legacy_allowlist;
#[path = "module_boundaries/plan_module_owns_generate_plans_and_frozen_verification.rs"]
mod plan_module_owns_generate_plans_and_frozen_verification;
#[path = "module_boundaries/kconfig_parser_parses_choice_entries.rs"]
mod kconfig_parser_parses_choice_entries;
#[path = "module_boundaries/kconfig_parser_parses_comment_entries.rs"]
mod kconfig_parser_parses_comment_entries;
#[path = "module_boundaries/kconfig_parser_parses_config_entries.rs"]
mod kconfig_parser_parses_config_entries;
#[path = "module_boundaries/kconfig_parser_parses_endchoice_markers.rs"]
mod kconfig_parser_parses_endchoice_markers;
#[path = "module_boundaries/kconfig_parser_parses_endif_markers.rs"]
mod kconfig_parser_parses_endif_markers;
#[path = "module_boundaries/kconfig_parser_parses_endmenu_markers.rs"]
mod kconfig_parser_parses_endmenu_markers;
#[path = "module_boundaries/kconfig_parser_parses_help_blocks.rs"]
mod kconfig_parser_parses_help_blocks;
#[path = "module_boundaries/kconfig_parser_parses_if_entries.rs"]
mod kconfig_parser_parses_if_entries;
#[path = "module_boundaries/kconfig_parser_parses_mainmenu_entries.rs"]
mod kconfig_parser_parses_mainmenu_entries;
#[path = "module_boundaries/kconfig_parser_parses_menu_entries.rs"]
mod kconfig_parser_parses_menu_entries;
#[path = "module_boundaries/kconfig_parser_parses_menuconfig_entries.rs"]
mod kconfig_parser_parses_menuconfig_entries;
#[path = "module_boundaries/kconfig_parser_parses_orsource_entries.rs"]
mod kconfig_parser_parses_orsource_entries;
#[path = "module_boundaries/kconfig_parser_parses_osource_entries.rs"]
mod kconfig_parser_parses_osource_entries;
#[path = "module_boundaries/kconfig_parser_parses_rsource_entries.rs"]
mod kconfig_parser_parses_rsource_entries;
#[path = "module_boundaries/kconfig_parser_parses_source_entries.rs"]
mod kconfig_parser_parses_source_entries;
#[path = "module_boundaries/kconfig_parser_preserves_formatting_lines.rs"]
mod kconfig_parser_preserves_formatting_lines;
#[path = "module_boundaries/kconfig_parser_preserves_line_comments.rs"]
mod kconfig_parser_preserves_line_comments;
#[path = "module_boundaries/kconfig_parser_preserves_unknown_syntax_as_skipped_site.rs"]
mod kconfig_parser_preserves_unknown_syntax_as_skipped_site;
#[path = "module_boundaries/kconfig_rewrite_module_uses_ast_and_tristate_boundaries.rs"]
mod kconfig_rewrite_module_uses_ast_and_tristate_boundaries;
#[path = "module_boundaries/kconfig_rewrite_removes_dead_select_edges_only_with_valid_source.rs"]
mod kconfig_rewrite_removes_dead_select_edges_only_with_valid_source;
#[path = "module_boundaries/kconfig_rewrite_removes_dead_imply_edges_only_with_valid_source.rs"]
mod kconfig_rewrite_removes_dead_imply_edges_only_with_valid_source;
#[path = "module_boundaries/kconfig_rewrite_removes_dead_symbol_definitions_only_with_solver_proof.rs"]
mod kconfig_rewrite_removes_dead_symbol_definitions_only_with_solver_proof;
#[path = "module_boundaries/kconfig_rewrite_simplifies_expressions_only_when_tristate_equivalent.rs"]
mod kconfig_rewrite_simplifies_expressions_only_when_tristate_equivalent;
#[path = "module_boundaries/kconfig_rewrite_preserves_unknown_expressions.rs"]
mod kconfig_rewrite_preserves_unknown_expressions;
#[path = "module_boundaries/kconfig_rewrite_preserves_symbols_used_by_live_arches.rs"]
mod kconfig_rewrite_preserves_symbols_used_by_live_arches;
#[path = "module_boundaries/kconfig_rewrite_preserves_abi_guard_symbols_unless_policy_allows_removal.rs"]
mod kconfig_rewrite_preserves_abi_guard_symbols_unless_policy_allows_removal;
#[path = "module_boundaries/kconfig_rewrite_preserves_prompt_text_unless_removing_full_symbol_block.rs"]
mod kconfig_rewrite_preserves_prompt_text_unless_removing_full_symbol_block;
#[path = "module_boundaries/kconfig_rewrite_preserves_help_text_unless_removing_full_symbol_block.rs"]
mod kconfig_rewrite_preserves_help_text_unless_removing_full_symbol_block;
#[path = "module_boundaries/kconfig_solver_report_is_emitted_as_reducer_artifact.rs"]
mod kconfig_solver_report_is_emitted_as_reducer_artifact;
#[path = "module_boundaries/kconfig_rewrite_report_is_emitted_as_reducer_artifact.rs"]
mod kconfig_rewrite_report_is_emitted_as_reducer_artifact;
#[path = "module_boundaries/kconfig_rewrite_removes_dead_source_lines_only_with_manifest_index_proof.rs"]
mod kconfig_rewrite_removes_dead_source_lines_only_with_manifest_index_proof;
#[path = "module_boundaries/kconfig_expression_solver_parses_and.rs"]
mod kconfig_expression_solver_parses_and;
#[path = "module_boundaries/kconfig_expression_logic_lives_in_expression_module.rs"]
mod kconfig_expression_logic_lives_in_expression_module;
#[path = "module_boundaries/kconfig_directive_parser_lives_in_parser_module.rs"]
mod kconfig_directive_parser_lives_in_parser_module;
#[path = "module_boundaries/kconfig_solver_analysis_lives_in_solver_module.rs"]
mod kconfig_solver_analysis_lives_in_solver_module;
#[path = "module_boundaries/kconfig_rewrite_application_lives_in_rewrite_module.rs"]
mod kconfig_rewrite_application_lives_in_rewrite_module;
#[path = "module_boundaries/kconfig_report_model_lives_in_report_module.rs"]
mod kconfig_report_model_lives_in_report_module;
#[path = "module_boundaries/kconfig_ast_tests_are_behavior_focused.rs"]
mod kconfig_ast_tests_are_behavior_focused;
#[path = "module_boundaries/kconfig_unit_tests_live_beside_owned_modules.rs"]
mod kconfig_unit_tests_live_beside_owned_modules;
#[path = "module_boundaries/kconfig_expression_solver_parses_or.rs"]
mod kconfig_expression_solver_parses_or;
#[path = "module_boundaries/kconfig_expression_solver_parses_not.rs"]
mod kconfig_expression_solver_parses_not;
#[path = "module_boundaries/kconfig_expression_solver_parses_equality.rs"]
mod kconfig_expression_solver_parses_equality;
#[path = "module_boundaries/kconfig_expression_solver_parses_inequality.rs"]
mod kconfig_expression_solver_parses_inequality;
#[path = "module_boundaries/kconfig_expression_solver_parses_symbol_references.rs"]
mod kconfig_expression_solver_parses_symbol_references;
#[path = "module_boundaries/kconfig_expression_solver_parses_y_literal.rs"]
mod kconfig_expression_solver_parses_y_literal;
#[path = "module_boundaries/kconfig_expression_solver_parses_m_literal.rs"]
mod kconfig_expression_solver_parses_m_literal;
#[path = "module_boundaries/kconfig_expression_solver_parses_n_literal.rs"]
mod kconfig_expression_solver_parses_n_literal;
#[path = "module_boundaries/kconfig_expression_solver_evaluates_tristate_min_max.rs"]
mod kconfig_expression_solver_evaluates_tristate_min_max;
#[path = "module_boundaries/kconfig_expression_solver_evaluates_visibility.rs"]
mod kconfig_expression_solver_evaluates_visibility;
#[path = "module_boundaries/kconfig_expression_solver_evaluates_profile_reachability.rs"]
mod kconfig_expression_solver_evaluates_profile_reachability;
#[path = "module_boundaries/kconfig_expression_solver_evaluates_removed_symbol_effect.rs"]
mod kconfig_expression_solver_evaluates_removed_symbol_effect;
#[path = "module_boundaries/kconfig_expression_solver_evaluates_defaults_after_removal.rs"]
mod kconfig_expression_solver_evaluates_defaults_after_removal;
#[path = "module_boundaries/kconfig_expression_solver_detects_symbols_reenabled_by_defaults.rs"]
mod kconfig_expression_solver_detects_symbols_reenabled_by_defaults;
#[path = "module_boundaries/kconfig_expression_solver_detects_removed_symbols_forced_by_select.rs"]
mod kconfig_expression_solver_detects_removed_symbols_forced_by_select;
#[path = "module_boundaries/kconfig_expression_solver_detects_removed_symbols_weakly_enabled_by_imply.rs"]
mod kconfig_expression_solver_detects_removed_symbols_weakly_enabled_by_imply;
#[path = "module_boundaries/kconfig_expression_solver_detects_impossible_choices.rs"]
mod kconfig_expression_solver_detects_impossible_choices;
#[path = "module_boundaries/kconfig_expression_solver_detects_empty_menus.rs"]
mod kconfig_expression_solver_detects_empty_menus;
#[path = "module_boundaries/kconfig_rewrite_cleans_empty_menus_only_with_solver_proof.rs"]
mod kconfig_rewrite_cleans_empty_menus_only_with_solver_proof;
#[path = "module_boundaries/kconfig_expression_solver_detects_orphaned_symbol_definitions.rs"]
mod kconfig_expression_solver_detects_orphaned_symbol_definitions;
#[path = "module_boundaries/kconfig_symbol_model_models_bool.rs"]
mod kconfig_symbol_model_models_bool;
#[path = "module_boundaries/kconfig_symbol_model_models_defaults.rs"]
mod kconfig_symbol_model_models_defaults;
#[path = "module_boundaries/kconfig_symbol_model_models_dependencies.rs"]
mod kconfig_symbol_model_models_dependencies;
#[path = "module_boundaries/kconfig_symbol_model_models_ranges.rs"]
mod kconfig_symbol_model_models_ranges;
#[path = "module_boundaries/kconfig_symbol_model_models_reverse_dependencies_through_select.rs"]
mod kconfig_symbol_model_models_reverse_dependencies_through_select;
#[path = "module_boundaries/kconfig_symbol_model_models_hex.rs"]
mod kconfig_symbol_model_models_hex;
#[path = "module_boundaries/kconfig_symbol_model_models_int.rs"]
mod kconfig_symbol_model_models_int;
#[path = "module_boundaries/kconfig_symbol_model_models_modules.rs"]
mod kconfig_symbol_model_models_modules;
#[path = "module_boundaries/kconfig_symbol_model_models_multiple_symbol_definitions.rs"]
mod kconfig_symbol_model_models_multiple_symbol_definitions;
#[path = "module_boundaries/kconfig_symbol_model_models_option.rs"]
mod kconfig_symbol_model_models_option;
#[path = "module_boundaries/kconfig_symbol_model_models_prompt_visibility.rs"]
mod kconfig_symbol_model_models_prompt_visibility;
#[path = "module_boundaries/kconfig_symbol_model_models_string.rs"]
mod kconfig_symbol_model_models_string;
#[path = "module_boundaries/kconfig_symbol_model_models_tristate.rs"]
mod kconfig_symbol_model_models_tristate;
#[path = "module_boundaries/kconfig_symbol_model_models_weak_reverse_dependencies_through_imply.rs"]
mod kconfig_symbol_model_models_weak_reverse_dependencies_through_imply;
#[path = "module_boundaries/kconfig_symbol_model_validates_type_consistency_across_definitions.rs"]
mod kconfig_symbol_model_validates_type_consistency_across_definitions;
#[path = "module_boundaries/kconfig_symbol_model_validates_prompt_consistency_policy.rs"]
mod kconfig_symbol_model_validates_prompt_consistency_policy;
#[path = "module_boundaries/kconfig_symbol_model_tracks_definition_source_locations.rs"]
mod kconfig_symbol_model_tracks_definition_source_locations;
#[path = "module_boundaries/kslim_config_is_project_root_model.rs"]
mod kslim_config_is_project_root_model;
#[path = "module_boundaries/model_module_owns_shared_value_models.rs"]
mod model_module_owns_shared_value_models;
#[path = "module_boundaries/module_boundary_tests_are_split_by_boundary.rs"]
mod module_boundary_tests_are_split_by_boundary;
#[path = "module_boundaries/output_config_is_project_output_repo_intent_model.rs"]
mod output_config_is_project_output_repo_intent_model;
#[path = "module_boundaries/output_repo_metadata_module_owns_metadata_schemas_and_io.rs"]
mod output_repo_metadata_module_owns_metadata_schemas_and_io;
#[path = "module_boundaries/output_repo_naming_module_owns_stable_output_names.rs"]
mod output_repo_naming_module_owns_stable_output_names;
#[path = "module_boundaries/output_repo_publish_module_does_not_know_upstream_resolution_policy.rs"]
mod output_repo_publish_module_does_not_know_upstream_resolution_policy;
#[path = "module_boundaries/output_repo_report_module_owns_report_names_paths_authority_and_copy.rs"]
mod output_repo_report_module_owns_report_names_paths_authority_and_copy;
#[path = "module_boundaries/output_repo_report_writer_module_owns_report_writes.rs"]
mod output_repo_report_writer_module_owns_report_writes;
#[path = "module_boundaries/output_repo_safety_module_owns_pre_mutation_boundary.rs"]
mod output_repo_safety_module_owns_pre_mutation_boundary;
#[path = "module_boundaries/output_repo_sync_module_owns_copying_only.rs"]
mod output_repo_sync_module_owns_copying_only;
#[path = "module_boundaries/output_repo_transaction_module_owns_init_and_preflight.rs"]
mod output_repo_transaction_module_owns_init_and_preflight;
#[path = "module_boundaries/output_repo_unit_tests_live_beside_owned_modules.rs"]
mod output_repo_unit_tests_live_beside_owned_modules;
#[path = "module_boundaries/paths_module_defines_lifecycle_path_wrappers.rs"]
mod paths_module_defines_lifecycle_path_wrappers;
#[path = "module_boundaries/performance_config_is_profile_performance_policy_model.rs"]
mod performance_config_is_profile_performance_policy_model;
#[path = "module_boundaries/primitive_rewrite_modules_do_not_depend_on_orchestration_policy_modules.rs"]
mod primitive_rewrite_modules_do_not_depend_on_orchestration_policy_modules;
#[path = "module_boundaries/prune_path_pruning_lives_in_path_module.rs"]
mod prune_path_pruning_lives_in_path_module;
#[path = "module_boundaries/prune_orphan_pruning_lives_in_orphan_module.rs"]
mod prune_orphan_pruning_lives_in_orphan_module;
#[path = "module_boundaries/prune_semantic_pruning_lives_in_semantic_module.rs"]
mod prune_semantic_pruning_lives_in_semantic_module;
#[path = "module_boundaries/prune_stale_reference_pruning_lives_in_stale_reference_module.rs"]
mod prune_stale_reference_pruning_lives_in_stale_reference_module;
#[path = "module_boundaries/prune_reporting_lives_in_report_module.rs"]
mod prune_reporting_lives_in_report_module;
#[path = "module_boundaries/profile_config_is_profile_file_user_intent_model.rs"]
mod profile_config_is_profile_file_user_intent_model;
#[path = "module_boundaries/public_header_removal_uses_explicit_abi_policy_boundary.rs"]
mod public_header_removal_uses_explicit_abi_policy_boundary;
#[path = "module_boundaries/publish_command_consumes_committed_output_metadata_only.rs"]
mod publish_command_consumes_committed_output_metadata_only;
#[path = "module_boundaries/publish_command_never_re_resolves_mutable_generate_inputs.rs"]
mod publish_command_never_re_resolves_mutable_generate_inputs;
#[path = "module_boundaries/published_snapshot_state_is_commit_only.rs"]
mod published_snapshot_state_is_commit_only;
#[path = "module_boundaries/published_state_requires_successful_commit_proof.rs"]
mod published_state_requires_successful_commit_proof;
#[path = "module_boundaries/reducer_attempt_state_is_attempt_only.rs"]
mod reducer_attempt_state_is_attempt_only;
#[path = "module_boundaries/reducer_config_is_reducer_policy_model.rs"]
mod reducer_config_is_reducer_policy_model;
#[path = "module_boundaries/reducer_context_actions_and_diagnostics_modules_define_reducer_boundaries.rs"]
mod reducer_context_actions_and_diagnostics_modules_define_reducer_boundaries;
#[path = "module_boundaries/reducer_engine_module_defines_fixed_point_loop_entrypoint.rs"]
mod reducer_engine_module_defines_fixed_point_loop_entrypoint;
#[path = "module_boundaries/reducer_mod_is_module_facade_only.rs"]
mod reducer_mod_is_module_facade_only;
#[path = "module_boundaries/reducer_pipeline_module_owns_fixed_pass_table.rs"]
mod reducer_pipeline_module_owns_fixed_pass_table;
#[path = "module_boundaries/reducer_report_module_emits_structured_reducer_reports.rs"]
mod reducer_report_module_emits_structured_reducer_reports;
#[path = "module_boundaries/reducer_result_module_owns_reducer_result_shape.rs"]
mod reducer_result_module_owns_reducer_result_shape;
#[path = "module_boundaries/report_rendering_is_separate_from_report_models.rs"]
mod report_rendering_is_separate_from_report_models;
#[path = "module_boundaries/removal_manifest_modules_define_manifest_boundaries.rs"]
mod removal_manifest_modules_define_manifest_boundaries;
#[path = "module_boundaries/report_config_is_profile_report_policy_model.rs"]
mod report_config_is_profile_report_policy_model;
#[path = "module_boundaries/requested_generate_state_is_request_only.rs"]
mod requested_generate_state_is_request_only;
#[path = "module_boundaries/resolved_candidate_state_is_plan_only.rs"]
mod resolved_candidate_state_is_plan_only;
#[path = "module_boundaries/runtime_matrix_config_is_profile_runtime_matrix_model.rs"]
mod runtime_matrix_config_is_profile_runtime_matrix_model;
#[path = "module_boundaries/runtime_module_owns_reachability_and_registration_proof.rs"]
mod runtime_module_owns_reachability_and_registration_proof;
#[path = "module_boundaries/runtime_registration_removal_requires_no_live_entry_point_proof.rs"]
mod runtime_registration_removal_requires_no_live_entry_point_proof;
#[path = "module_boundaries/rust_source_files_over_2000_lines_are_explicitly_justified.rs"]
mod rust_source_files_over_2000_lines_are_explicitly_justified;
#[path = "module_boundaries/security_config_is_profile_security_policy_model.rs"]
mod security_config_is_profile_security_policy_model;
#[path = "module_boundaries/security_policy_is_separate_from_implementation_details.rs"]
mod security_policy_is_separate_from_implementation_details;
#[path = "module_boundaries/selftest_modules_execute_tests_without_reducer_policy.rs"]
mod selftest_modules_execute_tests_without_reducer_policy;
#[path = "module_boundaries/slim_config_is_direct_removal_intent_model.rs"]
mod slim_config_is_direct_removal_intent_model;
#[path = "module_boundaries/source_scan_module_owns_c_family_scanning.rs"]
mod source_scan_module_owns_c_family_scanning;
#[path = "module_boundaries/stage_enums_are_logged_as_stable_names.rs"]
mod stage_enums_are_logged_as_stable_names;
#[path = "module_boundaries/stage_enums_are_stored_in_attempt_metadata.rs"]
mod stage_enums_are_stored_in_attempt_metadata;
#[path = "module_boundaries/stage_enums_are_stored_in_ci_summaries.rs"]
mod stage_enums_are_stored_in_ci_summaries;
#[path = "module_boundaries/stage_enums_are_stored_in_failure_reports.rs"]
mod stage_enums_are_stored_in_failure_reports;
#[path = "module_boundaries/stage_enums_are_stored_in_generate_reports.rs"]
mod stage_enums_are_stored_in_generate_reports;
#[path = "module_boundaries/state_module_owns_lifecycle_states.rs"]
mod state_module_owns_lifecycle_states;
#[path = "module_boundaries/strict_mode_blocks_feature_conflict_mutation.rs"]
mod strict_mode_blocks_feature_conflict_mutation;
#[path = "module_boundaries/tree_index_abi_index_lives_in_abi_index_module.rs"]
mod tree_index_abi_index_lives_in_abi_index_module;
#[path = "module_boundaries/tree_index_file_index_lives_in_file_index_module.rs"]
mod tree_index_file_index_lives_in_file_index_module;
#[path = "module_boundaries/tree_index_kbuild_index_lives_in_kbuild_index_module.rs"]
mod tree_index_kbuild_index_lives_in_kbuild_index_module;
#[path = "module_boundaries/tree_index_kconfig_index_lives_in_kconfig_index_module.rs"]
mod tree_index_kconfig_index_lives_in_kconfig_index_module;
#[path = "module_boundaries/tree_index_query_api_lives_in_query_module.rs"]
mod tree_index_query_api_lives_in_query_module;
#[path = "module_boundaries/tree_index_source_index_lives_in_source_index_module.rs"]
mod tree_index_source_index_lives_in_source_index_module;
#[path = "module_boundaries/tree_index_unit_tests_live_beside_owned_modules.rs"]
mod tree_index_unit_tests_live_beside_owned_modules;
#[path = "module_boundaries/tree_index_is_read_only_and_policy_free.rs"]
mod tree_index_is_read_only_and_policy_free;
