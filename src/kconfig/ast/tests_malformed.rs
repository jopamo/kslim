use super::*;

#[test]
fn parse_kconfig_document_rejects_malformed_config_headers() {
    let missing = parse_kconfig_document("config\n").unwrap_err();
    assert!(format!("{missing:#}").contains("missing a symbol"));

    let trailing = parse_kconfig_document("config FOO BAR\n").unwrap_err();
    assert!(format!("{trailing:#}").contains("unexpected trailing tokens"));

    let invalid = parse_kconfig_document("config FOO-BAR\n").unwrap_err();
    assert!(format!("{invalid:#}").contains("invalid Kconfig config symbol on line 1"));
}
#[test]
fn parse_kconfig_document_rejects_malformed_menuconfig_headers() {
    let missing = parse_kconfig_document("menuconfig\n").unwrap_err();
    assert!(format!("{missing:#}").contains("missing a symbol"));

    let trailing = parse_kconfig_document("menuconfig FOO BAR\n").unwrap_err();
    assert!(format!("{trailing:#}").contains("unexpected trailing tokens"));

    let invalid = parse_kconfig_document("menuconfig FOO-BAR\n").unwrap_err();
    assert!(format!("{invalid:#}").contains("invalid Kconfig menuconfig symbol on line 1"));
}
#[test]
fn parse_kconfig_document_rejects_malformed_choice_headers() {
    let trailing = parse_kconfig_document("choice FOO BAR\n").unwrap_err();
    assert!(format!("{trailing:#}").contains("unexpected trailing tokens"));

    let invalid = parse_kconfig_document("choice FOO-BAR\n").unwrap_err();
    assert!(format!("{invalid:#}").contains("invalid Kconfig choice symbol on line 1"));
}
#[test]
fn parse_kconfig_document_rejects_malformed_endchoice_markers() {
    let trailing = parse_kconfig_document("endchoice FOO\n").unwrap_err();
    assert!(format!("{trailing:#}").contains("unexpected trailing tokens"));
}
#[test]
fn parse_kconfig_document_rejects_malformed_menu_headers() {
    let missing = parse_kconfig_document("menu\n").unwrap_err();
    assert!(format!("{missing:#}").contains("missing a prompt"));

    let unquoted = parse_kconfig_document("menu DRIVERS\n").unwrap_err();
    assert!(format!("{unquoted:#}").contains("missing a quoted prompt"));

    let unterminated = parse_kconfig_document("menu \"Drivers\n").unwrap_err();
    assert!(format!("{unterminated:#}").contains("unterminated quoted prompt"));

    let trailing = parse_kconfig_document("menu \"Drivers\" EXTRA\n").unwrap_err();
    assert!(format!("{trailing:#}").contains("unexpected trailing tokens"));
}
#[test]
fn parse_kconfig_document_rejects_malformed_comment_headers() {
    let missing = parse_kconfig_document("comment\n").unwrap_err();
    assert!(format!("{missing:#}").contains("missing a prompt"));

    let whitespace = parse_kconfig_document("comment   \n").unwrap_err();
    assert!(format!("{whitespace:#}").contains("missing a prompt"));

    let unquoted = parse_kconfig_document("comment Drivers\n").unwrap_err();
    assert!(format!("{unquoted:#}").contains("missing a quoted prompt"));

    let unterminated = parse_kconfig_document("comment \"Drivers\n").unwrap_err();
    assert!(format!("{unterminated:#}").contains("unterminated quoted prompt"));

    let trailing = parse_kconfig_document("comment \"Drivers\" EXTRA\n").unwrap_err();
    assert!(format!("{trailing:#}").contains("unexpected trailing tokens"));
}
#[test]
fn parse_kconfig_document_rejects_malformed_mainmenu_headers() {
    let missing = parse_kconfig_document("mainmenu\n").unwrap_err();
    assert!(format!("{missing:#}").contains("missing a prompt"));

    let whitespace = parse_kconfig_document("mainmenu   \n").unwrap_err();
    assert!(format!("{whitespace:#}").contains("missing a prompt"));

    let unquoted = parse_kconfig_document("mainmenu Linux\n").unwrap_err();
    assert!(format!("{unquoted:#}").contains("missing a quoted prompt"));

    let unterminated = parse_kconfig_document("mainmenu \"Linux\n").unwrap_err();
    assert!(format!("{unterminated:#}").contains("unterminated quoted prompt"));

    let trailing = parse_kconfig_document("mainmenu \"Linux\" EXTRA\n").unwrap_err();
    assert!(format!("{trailing:#}").contains("unexpected trailing tokens"));
}
#[test]
fn parse_kconfig_document_rejects_malformed_endmenu_markers() {
    let trailing = parse_kconfig_document("endmenu EXTRA\n").unwrap_err();
    assert!(format!("{trailing:#}").contains("unexpected trailing tokens"));
}
#[test]
fn parse_kconfig_document_rejects_malformed_if_headers() {
    let missing = parse_kconfig_document("if\n").unwrap_err();
    assert!(format!("{missing:#}").contains("missing a condition"));

    let whitespace = parse_kconfig_document("if   \n").unwrap_err();
    assert!(format!("{whitespace:#}").contains("missing a condition"));
}
#[test]
fn parse_kconfig_document_rejects_malformed_endif_markers() {
    let trailing = parse_kconfig_document("endif EXTRA\n").unwrap_err();
    assert!(format!("{trailing:#}").contains("unexpected trailing tokens"));
}
#[test]
fn parse_kconfig_document_rejects_malformed_source_headers() {
    let missing = parse_kconfig_document("source\n").unwrap_err();
    assert!(format!("{missing:#}").contains("missing a path"));

    let whitespace = parse_kconfig_document("source   \n").unwrap_err();
    assert!(format!("{whitespace:#}").contains("missing a path"));

    let unquoted = parse_kconfig_document("source drivers/foo/Kconfig\n").unwrap_err();
    assert!(format!("{unquoted:#}").contains("missing a quoted path"));

    let empty = parse_kconfig_document("source \"\"\n").unwrap_err();
    assert!(format!("{empty:#}").contains("missing a path"));

    let unterminated = parse_kconfig_document("source \"drivers/foo/Kconfig\n").unwrap_err();
    assert!(format!("{unterminated:#}").contains("unterminated quoted path"));

    let trailing =
        parse_kconfig_document("source \"drivers/foo/Kconfig\" EXTRA\n").unwrap_err();
    assert!(format!("{trailing:#}").contains("unexpected trailing tokens"));
}
#[test]
fn parse_kconfig_document_rejects_malformed_rsource_headers() {
    let missing = parse_kconfig_document("rsource\n").unwrap_err();
    assert!(format!("{missing:#}").contains("missing a path"));

    let whitespace = parse_kconfig_document("rsource   \n").unwrap_err();
    assert!(format!("{whitespace:#}").contains("missing a path"));

    let unquoted = parse_kconfig_document("rsource subsys/Kconfig\n").unwrap_err();
    assert!(format!("{unquoted:#}").contains("missing a quoted path"));

    let empty = parse_kconfig_document("rsource \"\"\n").unwrap_err();
    assert!(format!("{empty:#}").contains("missing a path"));

    let unterminated = parse_kconfig_document("rsource \"subsys/Kconfig\n").unwrap_err();
    assert!(format!("{unterminated:#}").contains("unterminated quoted path"));

    let trailing = parse_kconfig_document("rsource \"subsys/Kconfig\" EXTRA\n").unwrap_err();
    assert!(format!("{trailing:#}").contains("unexpected trailing tokens"));
}
#[test]
fn parse_kconfig_document_rejects_malformed_osource_headers() {
    let missing = parse_kconfig_document("osource\n").unwrap_err();
    assert!(format!("{missing:#}").contains("missing a path"));

    let whitespace = parse_kconfig_document("osource   \n").unwrap_err();
    assert!(format!("{whitespace:#}").contains("missing a path"));

    let unquoted = parse_kconfig_document("osource drivers/optional/Kconfig\n").unwrap_err();
    assert!(format!("{unquoted:#}").contains("missing a quoted path"));

    let empty = parse_kconfig_document("osource \"\"\n").unwrap_err();
    assert!(format!("{empty:#}").contains("missing a path"));

    let unterminated =
        parse_kconfig_document("osource \"drivers/optional/Kconfig\n").unwrap_err();
    assert!(format!("{unterminated:#}").contains("unterminated quoted path"));

    let trailing =
        parse_kconfig_document("osource \"drivers/optional/Kconfig\" EXTRA\n").unwrap_err();
    assert!(format!("{trailing:#}").contains("unexpected trailing tokens"));
}
#[test]
fn parse_kconfig_document_rejects_malformed_orsource_headers() {
    let missing = parse_kconfig_document("orsource\n").unwrap_err();
    assert!(format!("{missing:#}").contains("missing a path"));

    let whitespace = parse_kconfig_document("orsource   \n").unwrap_err();
    assert!(format!("{whitespace:#}").contains("missing a path"));

    let unquoted = parse_kconfig_document("orsource optional/subsys/Kconfig\n").unwrap_err();
    assert!(format!("{unquoted:#}").contains("missing a quoted path"));

    let empty = parse_kconfig_document("orsource \"\"\n").unwrap_err();
    assert!(format!("{empty:#}").contains("missing a path"));

    let unterminated =
        parse_kconfig_document("orsource \"optional/subsys/Kconfig\n").unwrap_err();
    assert!(format!("{unterminated:#}").contains("unterminated quoted path"));

    let trailing =
        parse_kconfig_document("orsource \"optional/subsys/Kconfig\" EXTRA\n").unwrap_err();
    assert!(format!("{trailing:#}").contains("unexpected trailing tokens"));
}
