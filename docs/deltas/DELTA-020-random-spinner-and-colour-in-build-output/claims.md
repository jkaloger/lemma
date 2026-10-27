- [AC-1] Each task in a live `lemma build` run shows a spinner whose frames all belong to the built-in set of spinner styles.
  @ref crates/lemma/tests/live.rs#every_live_spinner_frame_is_from_the_built_in_styles
- [AC-2] All spinner frames shown for one task belong to a single style; the style does not change mid-task. Styles have disjoint frame sets, so a frame identifies its style.
  @ref crates/lemma/tests/live.rs#one_task_keeps_one_spinner_style
- [AC-3] [intent] The spinner style is picked once per task, drawn independently of the previous task's pick, so consecutive tasks may share or differ in style.
- [AC-4] When stdout is a terminal and `NO_COLOR` is unset, live build output colours the task id, the task status (running, passed, failed) and the claim ref lines in distinct colours.
  @ref crates/lemma/tests/live.rs#live_build_colours_task_id_status_and_ref_lines_distinctly
- [AC-5] When stdout is not a terminal, build output carries no colour escape codes and no spinner frames.
  @ref crates/lemma/tests/live.rs#piped_build_output_has_no_colour_or_spinner_frames
- [AC-6] When stdout is a terminal and `NO_COLOR` is set, build output carries no colour escape codes and the watch line still shows spinner frames.
  @ref crates/lemma/tests/live.rs#no_color_on_a_terminal_drops_colour_and_keeps_the_spinner
- [AC-7] Spinner choice and colour do not change the text of any diagnostic line or the exit code of `lemma build`.
