use super::*;

#[test]
fn parse_kconfig_document_tracks_definition_source_locations() {
    let document = parse_kconfig_document(concat!(
        "config LOCATION_CONFIG\n",
        "\tbool \"Location config\"\n",
        "\thelp\n",
        "\t  source location help\n",
        "menuconfig LOCATION_MENU\n",
        "\ttristate\n",
        "choice LOCATION_CHOICE\n",
        "\tbool \"Choice\"\n",
        "endchoice\n",
        "choice\n",
        "\tbool \"Anonymous\"\n",
        "endchoice\n",
    ))
    .unwrap();

    let locations = document
        .symbol_definition_source_locations()
        .collect::<Vec<_>>();
    let groups = document.symbol_definition_groups();
    let choice_group = groups
        .iter()
        .find(|group| group.symbol().as_str() == "LOCATION_CHOICE")
        .unwrap();

    assert_eq!(
        locations
            .iter()
            .map(|location| location.symbol().as_str())
            .collect::<Vec<_>>(),
        vec!["LOCATION_CONFIG", "LOCATION_MENU", "LOCATION_CHOICE"]
    );
    assert_eq!(
        locations
            .iter()
            .map(KconfigDefinitionSourceLocation::kind)
            .collect::<Vec<_>>(),
        vec![
            KconfigSymbolDefinitionKind::Config,
            KconfigSymbolDefinitionKind::Menuconfig,
            KconfigSymbolDefinitionKind::Choice
        ]
    );
    assert_eq!(
        locations
            .iter()
            .map(KconfigDefinitionSourceLocation::line)
            .collect::<Vec<_>>(),
        vec![1, 5, 7]
    );
    assert_eq!(
        locations
            .iter()
            .map(KconfigDefinitionSourceLocation::end_line)
            .collect::<Vec<_>>(),
        vec![4, 6, 8]
    );
    assert_eq!(locations[0].directive().text(), "config LOCATION_CONFIG");
    assert_eq!(choice_group.source_locations()[0].line(), 7);
    assert_eq!(
        document
            .symbol_definitions()
            .next()
            .unwrap()
            .source_location()
            .directive()
            .text(),
        "config LOCATION_CONFIG"
    );
}
