use super::*;

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeatureKconfigSelection {
    selector: KconfigSymbol,
    selected: KconfigSymbol,
}

#[allow(dead_code)]
impl FeatureKconfigSelection {
    pub(crate) fn new(selector: KconfigSymbol, selected: KconfigSymbol) -> Result<Self> {
        if selector == selected {
            anyhow::bail!(
                "feature Kconfig selection endpoints must be distinct: {}",
                selector.as_str()
            );
        }
        Ok(Self { selector, selected })
    }

    pub(crate) fn from_names(selector: &str, selected: &str) -> Result<Self> {
        Self::new(KconfigSymbol::new(selector)?, KconfigSymbol::new(selected)?)
    }

    pub(crate) fn selector(&self) -> &KconfigSymbol {
        &self.selector
    }

    pub(crate) fn selected(&self) -> &KconfigSymbol {
        &self.selected
    }

    pub(crate) fn stable_key(&self) -> String {
        format!(
            "select:{}->{}",
            self.selector.as_str(),
            self.selected.as_str()
        )
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeatureKbuildReference {
    referencer: KbuildObject,
    referenced: KbuildObject,
}

#[allow(dead_code)]
impl FeatureKbuildReference {
    pub(crate) fn new(referencer: KbuildObject, referenced: KbuildObject) -> Result<Self> {
        if referencer == referenced {
            anyhow::bail!(
                "feature kbuild reference endpoints must be distinct: {}",
                referencer.as_str()
            );
        }
        Ok(Self {
            referencer,
            referenced,
        })
    }

    pub(crate) fn from_names(referencer: &str, referenced: &str) -> Result<Self> {
        Self::new(
            KbuildObject::new(referencer)?,
            KbuildObject::new(referenced)?,
        )
    }

    pub(crate) fn referencer(&self) -> &KbuildObject {
        &self.referencer
    }

    pub(crate) fn referenced(&self) -> &KbuildObject {
        &self.referenced
    }

    pub(crate) fn stable_key(&self) -> String {
        format!(
            "kbuild_ref:{}->{}",
            self.referencer.as_str(),
            self.referenced.as_str()
        )
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeatureExportedSymbolConsumer {
    consumer: FeatureId,
    symbol: ExportedSymbol,
}

#[allow(dead_code)]
impl FeatureExportedSymbolConsumer {
    pub(crate) fn new(consumer: FeatureId, symbol: ExportedSymbol) -> Self {
        Self { consumer, symbol }
    }

    pub(crate) fn from_names(consumer: &str, symbol: &str) -> Result<Self> {
        Ok(Self::new(
            FeatureId::new(consumer)?,
            ExportedSymbol::new(symbol)?,
        ))
    }

    pub(crate) fn consumer(&self) -> &FeatureId {
        &self.consumer
    }

    pub(crate) fn symbol(&self) -> &ExportedSymbol {
        &self.symbol
    }

    pub(crate) fn stable_key(&self) -> String {
        format!(
            "exported_symbol_consumer:{}->{}",
            self.consumer.as_str(),
            self.symbol.as_str()
        )
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum FeatureDeviceId {
    DeviceCompatible(DeviceCompatible),
    AcpiId(AcpiId),
    PciId(PciId),
    UsbId(UsbId),
}

#[allow(dead_code)]
impl FeatureDeviceId {
    pub(crate) fn device_compatible(compatible: DeviceCompatible) -> Self {
        Self::DeviceCompatible(compatible)
    }

    pub(crate) fn acpi_id(id: AcpiId) -> Self {
        Self::AcpiId(id)
    }

    pub(crate) fn pci_id(id: PciId) -> Self {
        Self::PciId(id)
    }

    pub(crate) fn usb_id(id: UsbId) -> Self {
        Self::UsbId(id)
    }

    pub(crate) fn from_device_compatible_name(compatible: &str) -> Result<Self> {
        Ok(Self::device_compatible(DeviceCompatible::new(compatible)?))
    }

    pub(crate) fn from_acpi_id_name(id: &str) -> Result<Self> {
        Ok(Self::acpi_id(AcpiId::new(id)?))
    }

    pub(crate) fn from_pci_id_name(id: &str) -> Result<Self> {
        Ok(Self::pci_id(PciId::new(id)?))
    }

    pub(crate) fn from_usb_id_name(id: &str) -> Result<Self> {
        Ok(Self::usb_id(UsbId::new(id)?))
    }

    pub(crate) fn kind_name(&self) -> &'static str {
        match self {
            Self::DeviceCompatible(_) => "device_compatible",
            Self::AcpiId(_) => "acpi_id",
            Self::PciId(_) => "pci_id",
            Self::UsbId(_) => "usb_id",
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        match self {
            Self::DeviceCompatible(compatible) => compatible.as_str(),
            Self::AcpiId(id) => id.as_str(),
            Self::PciId(id) => id.as_str(),
            Self::UsbId(id) => id.as_str(),
        }
    }

    pub(crate) fn stable_key(&self) -> String {
        format!("{}:{}", self.kind_name(), self.as_str())
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeatureDeviceIdTableReference {
    table_owner: FeatureId,
    id: FeatureDeviceId,
}

#[allow(dead_code)]
impl FeatureDeviceIdTableReference {
    pub(crate) fn new(table_owner: FeatureId, id: FeatureDeviceId) -> Self {
        Self { table_owner, id }
    }

    pub(crate) fn from_device_compatible_names(
        table_owner: &str,
        compatible: &str,
    ) -> Result<Self> {
        Ok(Self::new(
            FeatureId::new(table_owner)?,
            FeatureDeviceId::from_device_compatible_name(compatible)?,
        ))
    }

    pub(crate) fn from_acpi_id_names(table_owner: &str, id: &str) -> Result<Self> {
        Ok(Self::new(
            FeatureId::new(table_owner)?,
            FeatureDeviceId::from_acpi_id_name(id)?,
        ))
    }

    pub(crate) fn from_pci_id_names(table_owner: &str, id: &str) -> Result<Self> {
        Ok(Self::new(
            FeatureId::new(table_owner)?,
            FeatureDeviceId::from_pci_id_name(id)?,
        ))
    }

    pub(crate) fn from_usb_id_names(table_owner: &str, id: &str) -> Result<Self> {
        Ok(Self::new(
            FeatureId::new(table_owner)?,
            FeatureDeviceId::from_usb_id_name(id)?,
        ))
    }

    pub(crate) fn table_owner(&self) -> &FeatureId {
        &self.table_owner
    }

    pub(crate) fn id(&self) -> &FeatureDeviceId {
        &self.id
    }

    pub(crate) fn stable_key(&self) -> String {
        format!(
            "device_id_table_ref:{}->{}",
            self.table_owner.as_str(),
            self.id.stable_key()
        )
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeatureUserspaceUapiReference {
    referrer: FeatureId,
    path: UapiPath,
}

#[allow(dead_code)]
impl FeatureUserspaceUapiReference {
    pub(crate) fn new(referrer: FeatureId, path: UapiPath) -> Self {
        Self { referrer, path }
    }

    pub(crate) fn from_names(referrer: &str, path: &str) -> Result<Self> {
        Ok(Self::new(
            FeatureId::new(referrer)?,
            UapiPath::new(PathBuf::from(path))?,
        ))
    }

    pub(crate) fn referrer(&self) -> &FeatureId {
        &self.referrer
    }

    pub(crate) fn path(&self) -> &UapiPath {
        &self.path
    }

    pub(crate) fn stable_key(&self) -> String {
        format!(
            "userspace_uapi_ref:{}->{}",
            self.referrer.as_str(),
            self.path.as_str()
        )
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeatureRuntimeRegistrationReachability {
    reachable_from: FeatureId,
    runtime_registration: RuntimeRegistrationSurface,
}

#[allow(dead_code)]
impl FeatureRuntimeRegistrationReachability {
    pub(crate) fn new(
        reachable_from: FeatureId,
        runtime_registration: RuntimeRegistrationSurface,
    ) -> Self {
        Self {
            reachable_from,
            runtime_registration,
        }
    }

    pub(crate) fn from_names(reachable_from: &str, runtime_registration: &str) -> Result<Self> {
        Ok(Self::new(
            FeatureId::new(reachable_from)?,
            RuntimeRegistrationSurface::new(runtime_registration)?,
        ))
    }

    pub(crate) fn reachable_from(&self) -> &FeatureId {
        &self.reachable_from
    }

    pub(crate) fn runtime_registration(&self) -> &RuntimeRegistrationSurface {
        &self.runtime_registration
    }

    pub(crate) fn stable_key(&self) -> String {
        format!(
            "runtime_registration_reachability:{}->{}",
            self.reachable_from.as_str(),
            self.runtime_registration.as_str()
        )
    }
}

#[allow(dead_code)]
impl FeatureConflictReport {
    pub(crate) fn from_profile(profile: &ProfileConfig) -> Result<Self> {
        let graph = FeatureGraph::from_profile(profile)?;
        Self::from_graph(&graph)
    }

    pub(crate) fn from_graph(graph: &FeatureGraph) -> Result<Self> {
        Self::from_graph_and_feature_facts(
            graph,
            std::iter::empty::<FeatureKconfigSelection>(),
            std::iter::empty::<FeatureKbuildReference>(),
            std::iter::empty::<FeatureExportedSymbolConsumer>(),
            std::iter::empty::<FeatureDeviceIdTableReference>(),
            std::iter::empty::<FeatureUserspaceUapiReference>(),
            std::iter::empty::<FeatureRuntimeRegistrationReachability>(),
        )
    }

    pub(crate) fn from_graph_and_kconfig_selections(
        graph: &FeatureGraph,
        selections: impl IntoIterator<Item = FeatureKconfigSelection>,
    ) -> Result<Self> {
        Self::from_graph_and_feature_facts(
            graph,
            selections,
            std::iter::empty::<FeatureKbuildReference>(),
            std::iter::empty::<FeatureExportedSymbolConsumer>(),
            std::iter::empty::<FeatureDeviceIdTableReference>(),
            std::iter::empty::<FeatureUserspaceUapiReference>(),
            std::iter::empty::<FeatureRuntimeRegistrationReachability>(),
        )
    }

    pub(crate) fn from_graph_and_kbuild_references(
        graph: &FeatureGraph,
        references: impl IntoIterator<Item = FeatureKbuildReference>,
    ) -> Result<Self> {
        Self::from_graph_and_feature_facts(
            graph,
            std::iter::empty::<FeatureKconfigSelection>(),
            references,
            std::iter::empty::<FeatureExportedSymbolConsumer>(),
            std::iter::empty::<FeatureDeviceIdTableReference>(),
            std::iter::empty::<FeatureUserspaceUapiReference>(),
            std::iter::empty::<FeatureRuntimeRegistrationReachability>(),
        )
    }

    pub(crate) fn from_graph_and_exported_symbol_consumers(
        graph: &FeatureGraph,
        consumers: impl IntoIterator<Item = FeatureExportedSymbolConsumer>,
    ) -> Result<Self> {
        Self::from_graph_and_feature_facts(
            graph,
            std::iter::empty::<FeatureKconfigSelection>(),
            std::iter::empty::<FeatureKbuildReference>(),
            consumers,
            std::iter::empty::<FeatureDeviceIdTableReference>(),
            std::iter::empty::<FeatureUserspaceUapiReference>(),
            std::iter::empty::<FeatureRuntimeRegistrationReachability>(),
        )
    }

    pub(crate) fn from_graph_and_device_id_table_references(
        graph: &FeatureGraph,
        references: impl IntoIterator<Item = FeatureDeviceIdTableReference>,
    ) -> Result<Self> {
        Self::from_graph_and_feature_facts(
            graph,
            std::iter::empty::<FeatureKconfigSelection>(),
            std::iter::empty::<FeatureKbuildReference>(),
            std::iter::empty::<FeatureExportedSymbolConsumer>(),
            references,
            std::iter::empty::<FeatureUserspaceUapiReference>(),
            std::iter::empty::<FeatureRuntimeRegistrationReachability>(),
        )
    }

    pub(crate) fn from_graph_and_userspace_uapi_references(
        graph: &FeatureGraph,
        references: impl IntoIterator<Item = FeatureUserspaceUapiReference>,
    ) -> Result<Self> {
        Self::from_graph_and_feature_facts(
            graph,
            std::iter::empty::<FeatureKconfigSelection>(),
            std::iter::empty::<FeatureKbuildReference>(),
            std::iter::empty::<FeatureExportedSymbolConsumer>(),
            std::iter::empty::<FeatureDeviceIdTableReference>(),
            references,
            std::iter::empty::<FeatureRuntimeRegistrationReachability>(),
        )
    }

    pub(crate) fn from_graph_and_runtime_registration_reachability(
        graph: &FeatureGraph,
        reachability: impl IntoIterator<Item = FeatureRuntimeRegistrationReachability>,
    ) -> Result<Self> {
        Self::from_graph_and_feature_facts(
            graph,
            std::iter::empty::<FeatureKconfigSelection>(),
            std::iter::empty::<FeatureKbuildReference>(),
            std::iter::empty::<FeatureExportedSymbolConsumer>(),
            std::iter::empty::<FeatureDeviceIdTableReference>(),
            std::iter::empty::<FeatureUserspaceUapiReference>(),
            reachability,
        )
    }

    pub(crate) fn from_graph_and_feature_facts(
        graph: &FeatureGraph,
        selections: impl IntoIterator<Item = FeatureKconfigSelection>,
        references: impl IntoIterator<Item = FeatureKbuildReference>,
        consumers: impl IntoIterator<Item = FeatureExportedSymbolConsumer>,
        table_references: impl IntoIterator<Item = FeatureDeviceIdTableReference>,
        uapi_references: impl IntoIterator<Item = FeatureUserspaceUapiReference>,
        runtime_reachability: impl IntoIterator<Item = FeatureRuntimeRegistrationReachability>,
    ) -> Result<Self> {
        let mut conflicts = removed_feature_live_dependency_conflicts(graph)?;
        conflicts.extend(removed_feature_live_kconfig_selection_conflicts(
            graph, selections,
        )?);
        conflicts.extend(removed_feature_live_kbuild_reference_conflicts(
            graph, references,
        )?);
        conflicts.extend(removed_feature_exported_symbol_live_consumer_conflicts(
            graph, consumers,
        )?);
        conflicts.extend(removed_feature_device_id_live_table_conflicts(
            graph,
            table_references,
        )?);
        conflicts.extend(removed_feature_uapi_live_userspace_reference_conflicts(
            graph,
            uapi_references,
        )?);
        conflicts.extend(removed_feature_runtime_registration_reachable_conflicts(
            graph,
            runtime_reachability,
        )?);
        conflicts.extend(removed_feature_shared_file_conflicts(graph)?);
        Self::new(conflicts)
    }
}

#[allow(dead_code)]
fn removed_feature_live_dependency_conflicts(graph: &FeatureGraph) -> Result<Vec<FeatureConflict>> {
    let mut conflicts = Vec::new();
    for edge in graph.edges() {
        if edge.kind() != FeatureEdgeKind::Dependency {
            continue;
        }

        let live_consumer = graph
            .get(edge.from())
            .expect("feature graph should validate dependency source feature");
        let removed_dependency = graph
            .get(edge.to())
            .expect("feature graph should validate dependency target feature");

        if live_consumer.intent().action != FeatureIntentAction::Preserve
            || removed_dependency.intent().action != FeatureIntentAction::Remove
        {
            continue;
        }

        conflicts.push(FeatureConflict::new(
            FeatureConflictKind::RemovedFeatureOwnsLiveDependency,
            edge.to().clone(),
            FeatureOwnershipSubject::new(format!("feature:{}", edge.from().as_str()))?,
            format!(
                "removed feature '{}' is required by live feature '{}'",
                edge.to().as_str(),
                edge.from().as_str()
            ),
            format!(
                "preserve feature '{}' or remove live consumer '{}'",
                edge.to().as_str(),
                edge.from().as_str()
            ),
        )?);
    }
    Ok(conflicts)
}

fn removed_feature_live_kconfig_selection_conflicts(
    graph: &FeatureGraph,
    selections: impl IntoIterator<Item = FeatureKconfigSelection>,
) -> Result<Vec<FeatureConflict>> {
    let kconfig = FeatureKconfigResolution::from_graph(graph);
    let mut removed_features_by_symbol: BTreeMap<KconfigSymbol, Vec<FeatureId>> = BTreeMap::new();
    let mut live_features_by_symbol: BTreeMap<KconfigSymbol, Vec<FeatureId>> = BTreeMap::new();

    for symbol in kconfig.symbols() {
        if symbol.kind().is_removal() {
            removed_features_by_symbol
                .entry(symbol.symbol().clone())
                .or_default()
                .push(symbol.feature().clone());
        } else if symbol.kind().is_preservation() {
            live_features_by_symbol
                .entry(symbol.symbol().clone())
                .or_default()
                .push(symbol.feature().clone());
        }
    }

    let mut conflicts_by_key = BTreeMap::new();
    for selection in selections {
        if !live_features_by_symbol.contains_key(selection.selector()) {
            continue;
        }
        let Some(removed_features) = removed_features_by_symbol.get(selection.selected()) else {
            continue;
        };

        for removed_feature in removed_features {
            let conflict = FeatureConflict::new(
                FeatureConflictKind::RemovedFeatureSelectedByLiveKconfig,
                removed_feature.clone(),
                FeatureOwnershipSubject::new(format!("kconfig:{}", selection.selected().as_str()))?,
                format!(
                    "removed feature '{}' is still selected by live Kconfig symbol '{}'",
                    removed_feature.as_str(),
                    selection.selector().as_str()
                ),
                format!(
                    "remove the '{}' selector or preserve feature '{}'",
                    selection.selector().as_str(),
                    removed_feature.as_str()
                ),
            )?;
            conflicts_by_key
                .entry(conflict.stable_key())
                .or_insert(conflict);
        }
    }

    Ok(conflicts_by_key.into_values().collect())
}

fn removed_feature_live_kbuild_reference_conflicts(
    graph: &FeatureGraph,
    references: impl IntoIterator<Item = FeatureKbuildReference>,
) -> Result<Vec<FeatureConflict>> {
    let kbuild = FeatureKbuildResolution::from_graph(graph)?;
    let mut removed_features_by_object: BTreeMap<KbuildObject, Vec<FeatureId>> = BTreeMap::new();
    let mut live_features_by_object: BTreeMap<KbuildObject, Vec<FeatureId>> = BTreeMap::new();

    for object in kbuild.objects() {
        if object.kind().is_removal() {
            removed_features_by_object
                .entry(object.object().clone())
                .or_default()
                .push(object.feature().clone());
        } else if object.kind().is_preservation() {
            live_features_by_object
                .entry(object.object().clone())
                .or_default()
                .push(object.feature().clone());
        }
    }

    let mut conflicts_by_key = BTreeMap::new();
    for reference in references {
        if features_covering_kbuild_object(&live_features_by_object, reference.referencer())
            .is_empty()
        {
            continue;
        }

        for removed_feature in
            features_covering_kbuild_object(&removed_features_by_object, reference.referenced())
        {
            let conflict = FeatureConflict::new(
                FeatureConflictKind::RemovedFeatureReferencedByLiveKbuild,
                removed_feature,
                FeatureOwnershipSubject::new(format!(
                    "kbuild:{}",
                    reference.referenced().as_str()
                ))?,
                format!(
                    "removed feature is still referenced by live kbuild object '{}'",
                    reference.referencer().as_str()
                ),
                format!(
                    "remove the '{}' kbuild reference or preserve the removed feature",
                    reference.referencer().as_str()
                ),
            )?;
            conflicts_by_key
                .entry(conflict.stable_key())
                .or_insert(conflict);
        }
    }

    Ok(conflicts_by_key.into_values().collect())
}

fn features_covering_kbuild_object(
    features_by_object: &BTreeMap<KbuildObject, Vec<FeatureId>>,
    object: &KbuildObject,
) -> Vec<FeatureId> {
    let mut features = Vec::new();
    for (owned_object, owned_features) in features_by_object {
        if kbuild_object_covers(owned_object, object) {
            features.extend(owned_features.iter().cloned());
        }
    }
    features.sort();
    features.dedup();
    features
}

fn kbuild_object_covers(owner: &KbuildObject, object: &KbuildObject) -> bool {
    owner == object || (owner.is_directory_ref() && object.as_str().starts_with(owner.as_str()))
}

fn removed_feature_exported_symbol_live_consumer_conflicts(
    graph: &FeatureGraph,
    consumers: impl IntoIterator<Item = FeatureExportedSymbolConsumer>,
) -> Result<Vec<FeatureConflict>> {
    let exported_symbols = FeatureExportedSymbolResolution::from_graph(graph);
    let mut removed_features_by_symbol: BTreeMap<ExportedSymbol, Vec<FeatureId>> = BTreeMap::new();
    let mut live_features: BTreeMap<FeatureId, ()> = BTreeMap::new();

    for node in graph.nodes() {
        if node.intent().action == FeatureIntentAction::Preserve {
            live_features.insert(node.id().clone(), ());
        }
    }

    for symbol in exported_symbols.symbols() {
        if symbol.kind().is_removal() {
            removed_features_by_symbol
                .entry(symbol.symbol().clone())
                .or_default()
                .push(symbol.feature().clone());
        }
    }

    let mut conflicts_by_key = BTreeMap::new();
    for consumer in consumers {
        if !live_features.contains_key(consumer.consumer()) {
            continue;
        }
        let Some(removed_features) = removed_features_by_symbol.get(consumer.symbol()) else {
            continue;
        };

        for removed_feature in removed_features {
            let conflict = FeatureConflict::new(
                FeatureConflictKind::RemovedFeatureExportsConsumedSymbol,
                removed_feature.clone(),
                FeatureOwnershipSubject::new(format!("symbol:{}", consumer.symbol().as_str()))?,
                format!(
                    "removed feature exports symbol '{}' consumed by live code",
                    consumer.symbol().as_str()
                ),
                format!(
                    "remove live consumers of '{}' or preserve feature '{}'",
                    consumer.symbol().as_str(),
                    removed_feature.as_str()
                ),
            )?;
            conflicts_by_key
                .entry(conflict.stable_key())
                .or_insert(conflict);
        }
    }

    Ok(conflicts_by_key.into_values().collect())
}

fn removed_feature_device_id_live_table_conflicts(
    graph: &FeatureGraph,
    references: impl IntoIterator<Item = FeatureDeviceIdTableReference>,
) -> Result<Vec<FeatureConflict>> {
    let removed_features_by_id = removed_feature_device_ids_by_id(graph);
    let mut live_features: BTreeMap<FeatureId, ()> = BTreeMap::new();

    for node in graph.nodes() {
        if node.intent().action == FeatureIntentAction::Preserve {
            live_features.insert(node.id().clone(), ());
        }
    }

    let mut conflicts_by_key = BTreeMap::new();
    for reference in references {
        if !live_features.contains_key(reference.table_owner()) {
            continue;
        }
        let Some(removed_features) = removed_features_by_id.get(reference.id()) else {
            continue;
        };

        for removed_feature in removed_features {
            let conflict = FeatureConflict::new(
                FeatureConflictKind::RemovedFeatureDeviceIdReferencedByLiveTable,
                removed_feature.clone(),
                FeatureOwnershipSubject::new(reference.id().stable_key())?,
                format!(
                    "removed feature owns device ID '{}' referenced by live feature '{}' table",
                    reference.id().stable_key(),
                    reference.table_owner().as_str()
                ),
                format!(
                    "remove '{}' table reference to '{}' or preserve feature '{}'",
                    reference.table_owner().as_str(),
                    reference.id().stable_key(),
                    removed_feature.as_str()
                ),
            )?;
            conflicts_by_key
                .entry(conflict.stable_key())
                .or_insert(conflict);
        }
    }

    Ok(conflicts_by_key.into_values().collect())
}

fn removed_feature_device_ids_by_id(
    graph: &FeatureGraph,
) -> BTreeMap<FeatureDeviceId, Vec<FeatureId>> {
    let mut removed_features_by_id: BTreeMap<FeatureDeviceId, Vec<FeatureId>> = BTreeMap::new();

    for compatible in FeatureDeviceCompatibleResolution::from_graph(graph).compatibles() {
        if compatible.kind().is_removal() {
            removed_features_by_id
                .entry(FeatureDeviceId::device_compatible(
                    compatible.compatible().clone(),
                ))
                .or_default()
                .push(compatible.feature().clone());
        }
    }

    for id in FeatureAcpiIdResolution::from_graph(graph).ids() {
        if id.kind().is_removal() {
            removed_features_by_id
                .entry(FeatureDeviceId::acpi_id(id.id().clone()))
                .or_default()
                .push(id.feature().clone());
        }
    }

    for id in FeaturePciIdResolution::from_graph(graph).ids() {
        if id.kind().is_removal() {
            removed_features_by_id
                .entry(FeatureDeviceId::pci_id(id.id().clone()))
                .or_default()
                .push(id.feature().clone());
        }
    }

    for id in FeatureUsbIdResolution::from_graph(graph).ids() {
        if id.kind().is_removal() {
            removed_features_by_id
                .entry(FeatureDeviceId::usb_id(id.id().clone()))
                .or_default()
                .push(id.feature().clone());
        }
    }

    for features in removed_features_by_id.values_mut() {
        features.sort();
        features.dedup();
    }

    removed_features_by_id
}

fn removed_feature_uapi_live_userspace_reference_conflicts(
    graph: &FeatureGraph,
    references: impl IntoIterator<Item = FeatureUserspaceUapiReference>,
) -> Result<Vec<FeatureConflict>> {
    let uapi_headers = FeatureUapiHeaderResolution::from_graph(graph)?;
    let mut removed_features_by_path: BTreeMap<UapiPath, Vec<FeatureId>> = BTreeMap::new();
    let mut live_features: BTreeMap<FeatureId, ()> = BTreeMap::new();

    for node in graph.nodes() {
        if node.intent().action == FeatureIntentAction::Preserve {
            live_features.insert(node.id().clone(), ());
        }
    }

    for header in uapi_headers.headers() {
        if header.kind().is_removal() {
            removed_features_by_path
                .entry(header.header().clone())
                .or_default()
                .push(header.feature().clone());
        }
    }

    let mut conflicts_by_key = BTreeMap::new();
    for reference in references {
        if !live_features.contains_key(reference.referrer()) {
            continue;
        }
        let Some(removed_features) = removed_features_by_path.get(reference.path()) else {
            continue;
        };

        for removed_feature in removed_features {
            let conflict = FeatureConflict::new(
                FeatureConflictKind::RemovedFeatureUapiReferencedByUserspaceFacingCode,
                removed_feature.clone(),
                FeatureOwnershipSubject::new(format!("uapi_header:{}", reference.path().as_str()))?,
                format!(
                    "removed feature UAPI '{}' is referenced by live userspace-facing code '{}'",
                    reference.path().as_str(),
                    reference.referrer().as_str()
                ),
                format!(
                    "remove '{}' userspace UAPI reference to '{}' or preserve feature '{}'",
                    reference.referrer().as_str(),
                    reference.path().as_str(),
                    removed_feature.as_str()
                ),
            )?;
            conflicts_by_key
                .entry(conflict.stable_key())
                .or_insert(conflict);
        }
    }

    Ok(conflicts_by_key.into_values().collect())
}

fn removed_feature_runtime_registration_reachable_conflicts(
    graph: &FeatureGraph,
    reachability: impl IntoIterator<Item = FeatureRuntimeRegistrationReachability>,
) -> Result<Vec<FeatureConflict>> {
    let runtime_registrations = FeatureRuntimeRegistrationResolution::from_graph(graph);
    let mut removed_features_by_registration: BTreeMap<RuntimeRegistrationSurface, Vec<FeatureId>> =
        BTreeMap::new();
    let mut live_features: BTreeMap<FeatureId, ()> = BTreeMap::new();

    for node in graph.nodes() {
        if node.intent().action == FeatureIntentAction::Preserve {
            live_features.insert(node.id().clone(), ());
        }
    }

    for runtime_registration in runtime_registrations.runtime_registrations() {
        if runtime_registration.kind().is_removal() {
            removed_features_by_registration
                .entry(runtime_registration.runtime_registration().clone())
                .or_default()
                .push(runtime_registration.feature().clone());
        }
    }

    let mut conflicts_by_key = BTreeMap::new();
    for reachable in reachability {
        if !live_features.contains_key(reachable.reachable_from()) {
            continue;
        }
        let Some(removed_features) =
            removed_features_by_registration.get(reachable.runtime_registration())
        else {
            continue;
        };

        for removed_feature in removed_features {
            let conflict = FeatureConflict::new(
                FeatureConflictKind::RemovedFeatureRuntimeRegistrationReachable,
                removed_feature.clone(),
                FeatureOwnershipSubject::new(format!(
                    "runtime_registration_surface:{}",
                    reachable.runtime_registration().as_str()
                ))?,
                format!(
                    "removed feature runtime registration '{}' is still reachable from live feature '{}'",
                    reachable.runtime_registration().as_str(),
                    reachable.reachable_from().as_str()
                ),
                format!(
                    "remove '{}' runtime reachability to '{}' or preserve feature '{}'",
                    reachable.reachable_from().as_str(),
                    reachable.runtime_registration().as_str(),
                    removed_feature.as_str()
                ),
            )?;
            conflicts_by_key
                .entry(conflict.stable_key())
                .or_insert(conflict);
        }
    }

    Ok(conflicts_by_key.into_values().collect())
}

fn removed_feature_shared_file_conflicts(graph: &FeatureGraph) -> Result<Vec<FeatureConflict>> {
    let path_resolution = FeaturePathResolution::from_graph(graph);
    let removed_paths = path_resolution
        .paths()
        .iter()
        .filter(|path| path.kind().is_removal())
        .collect::<Vec<_>>();
    let preserved_paths = path_resolution
        .paths()
        .iter()
        .filter(|path| path.kind().is_preservation())
        .collect::<Vec<_>>();

    let mut conflicts_by_key = BTreeMap::new();
    for removed in &removed_paths {
        for preserved in &preserved_paths {
            if !feature_paths_overlap(removed.path(), preserved.path()) {
                continue;
            }
            let shared_path = shared_feature_path_subject(removed.path(), preserved.path())
                .expect("overlapping feature paths should have a shared path subject");
            let conflict = FeatureConflict::new(
                FeatureConflictKind::SharedFileBetweenRemovedAndPreservedFeatures,
                removed.feature().clone(),
                FeatureOwnershipSubject::new(format!(
                    "path:{}",
                    shared_path.as_path().to_string_lossy()
                ))?,
                format!(
                    "removed feature '{}' shares path '{}' with preserved feature '{}'",
                    removed.feature().as_str(),
                    shared_path.as_path().to_string_lossy(),
                    preserved.feature().as_str()
                ),
                format!(
                    "split shared path '{}', narrow feature roots, or preserve feature '{}'",
                    shared_path.as_path().to_string_lossy(),
                    removed.feature().as_str()
                ),
            )?;
            conflicts_by_key
                .entry(conflict.stable_key())
                .or_insert(conflict);
        }
    }

    Ok(conflicts_by_key.into_values().collect())
}

fn feature_paths_overlap(left: &RelativeKernelPath, right: &RelativeKernelPath) -> bool {
    shared_feature_path_subject(left, right).is_some()
}

fn shared_feature_path_subject<'a>(
    removed: &'a RelativeKernelPath,
    preserved: &'a RelativeKernelPath,
) -> Option<&'a RelativeKernelPath> {
    if crate::path_policy::normalized_relative_path_covers(removed.as_path(), preserved.as_path()) {
        Some(preserved)
    } else if crate::path_policy::normalized_relative_path_covers(
        preserved.as_path(),
        removed.as_path(),
    ) {
        Some(removed)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn intent(action: FeatureIntentAction, name: &str) -> FeatureIntent {
        FeatureIntent::from_config(
            action,
            name,
            &FeatureIntentConfig {
                roots: vec![format!("drivers/{name}")],
                ..FeatureIntentConfig::default()
            },
        )
        .unwrap()
    }

    #[test]
    fn reports_removed_dependency_owned_by_live_feature_edge() {
        let graph = FeatureGraph::with_edges(
            [
                intent(FeatureIntentAction::Remove, "bluetooth"),
                intent(FeatureIntentAction::Preserve, "rfkill"),
                intent(FeatureIntentAction::Remove, "wifi"),
            ],
            [
                FeatureEdge::from_names(FeatureEdgeKind::Dependency, "rfkill", "bluetooth")
                    .unwrap(),
                FeatureEdge::from_names(FeatureEdgeKind::Dependency, "bluetooth", "rfkill")
                    .unwrap(),
                FeatureEdge::from_names(FeatureEdgeKind::Conflict, "rfkill", "wifi").unwrap(),
            ],
        )
        .unwrap();

        let report = FeatureConflictReport::from_graph(&graph).unwrap();

        assert_eq!(report.len(), 1);
        assert!(report.has_blocking_conflicts());
        let conflict = &report.conflicts()[0];
        assert_eq!(
            conflict.kind(),
            FeatureConflictKind::RemovedFeatureOwnsLiveDependency
        );
        assert_eq!(conflict.feature().as_str(), "bluetooth");
        assert_eq!(conflict.subject().as_str(), "feature:rfkill");
        assert_eq!(
            conflict.stable_key(),
            "removed_feature_owns_live_dependency:bluetooth:feature:rfkill"
        );
        assert_eq!(
            conflict.summary(),
            "removed feature 'bluetooth' is required by live feature 'rfkill'"
        );
        assert_eq!(
            conflict.suggested_action(),
            "preserve feature 'bluetooth' or remove live consumer 'rfkill'"
        );
    }

    #[test]
    fn ignores_dependency_edges_without_removed_target_and_live_source() {
        let graph = FeatureGraph::with_edges(
            [
                intent(FeatureIntentAction::Remove, "bluetooth"),
                intent(FeatureIntentAction::Remove, "rfkill"),
                intent(FeatureIntentAction::Preserve, "netfilter"),
                intent(FeatureIntentAction::Preserve, "nftables"),
            ],
            [
                FeatureEdge::from_names(FeatureEdgeKind::Dependency, "bluetooth", "rfkill")
                    .unwrap(),
                FeatureEdge::from_names(FeatureEdgeKind::Dependency, "bluetooth", "netfilter")
                    .unwrap(),
                FeatureEdge::from_names(FeatureEdgeKind::Dependency, "netfilter", "nftables")
                    .unwrap(),
            ],
        )
        .unwrap();

        let report = FeatureConflictReport::from_graph(&graph).unwrap();

        assert!(report.is_empty());
    }

    #[test]
    fn reports_removed_feature_selected_by_live_kconfig_symbol() {
        let graph = FeatureGraph::new([
            FeatureIntent::from_config(
                FeatureIntentAction::Remove,
                "bluetooth",
                &FeatureIntentConfig {
                    configs: vec![String::from("BT")],
                    ..FeatureIntentConfig::default()
                },
            )
            .unwrap(),
            FeatureIntent::from_config(
                FeatureIntentAction::Preserve,
                "rfkill",
                &FeatureIntentConfig {
                    configs: vec![String::from("RFKILL")],
                    ..FeatureIntentConfig::default()
                },
            )
            .unwrap(),
            FeatureIntent::from_config(
                FeatureIntentAction::Remove,
                "wifi",
                &FeatureIntentConfig {
                    configs: vec![String::from("WIFI")],
                    ..FeatureIntentConfig::default()
                },
            )
            .unwrap(),
        ])
        .unwrap();

        let report = FeatureConflictReport::from_graph_and_kconfig_selections(
            &graph,
            [
                FeatureKconfigSelection::from_names("RFKILL", "BT").unwrap(),
                FeatureKconfigSelection::from_names("BT", "WIFI").unwrap(),
                FeatureKconfigSelection::from_names("RFKILL", "UNKNOWN").unwrap(),
            ],
        )
        .unwrap();

        assert_eq!(report.len(), 1);
        let conflict = &report.conflicts()[0];
        assert_eq!(
            conflict.kind(),
            FeatureConflictKind::RemovedFeatureSelectedByLiveKconfig
        );
        assert_eq!(conflict.feature().as_str(), "bluetooth");
        assert_eq!(conflict.subject().as_str(), "kconfig:BT");
        assert_eq!(
            conflict.stable_key(),
            "removed_feature_selected_by_live_kconfig:bluetooth:kconfig:BT"
        );
        assert_eq!(
            conflict.summary(),
            "removed feature 'bluetooth' is still selected by live Kconfig symbol 'RFKILL'"
        );
        assert_eq!(
            conflict.suggested_action(),
            "remove the 'RFKILL' selector or preserve feature 'bluetooth'"
        );
    }

    #[test]
    fn kconfig_selection_facts_are_stable_and_validate_endpoints() {
        let selection = FeatureKconfigSelection::from_names("RFKILL", "BT").unwrap();

        assert_eq!(selection.selector().as_str(), "RFKILL");
        assert_eq!(selection.selected().as_str(), "BT");
        assert_eq!(selection.stable_key(), "select:RFKILL->BT");

        let self_select = FeatureKconfigSelection::from_names("BT", "BT").unwrap_err();
        assert!(format!("{self_select:#}").contains("Kconfig selection endpoints must be distinct"));
    }

    #[test]
    fn reports_removed_feature_referenced_by_live_kbuild_object() {
        let graph = FeatureGraph::new([
            FeatureIntent::from_config(
                FeatureIntentAction::Remove,
                "bluetooth",
                &FeatureIntentConfig {
                    roots: vec![String::from("drivers/bluetooth")],
                    ..FeatureIntentConfig::default()
                },
            )
            .unwrap(),
            FeatureIntent::from_config(
                FeatureIntentAction::Preserve,
                "netfilter",
                &FeatureIntentConfig {
                    roots: vec![String::from("net/netfilter")],
                    ..FeatureIntentConfig::default()
                },
            )
            .unwrap(),
            FeatureIntent::from_config(
                FeatureIntentAction::Remove,
                "wifi",
                &FeatureIntentConfig {
                    roots: vec![String::from("drivers/net/wireless/wifi.c")],
                    ..FeatureIntentConfig::default()
                },
            )
            .unwrap(),
        ])
        .unwrap();

        let report = FeatureConflictReport::from_graph_and_kbuild_references(
            &graph,
            [
                FeatureKbuildReference::from_names(
                    "net/netfilter/core.o",
                    "drivers/bluetooth/btusb.o",
                )
                .unwrap(),
                FeatureKbuildReference::from_names(
                    "drivers/bluetooth/btusb.o",
                    "drivers/net/wireless/wifi.o",
                )
                .unwrap(),
                FeatureKbuildReference::from_names(
                    "net/netfilter/core.o",
                    "drivers/net/ethernet/live.o",
                )
                .unwrap(),
            ],
        )
        .unwrap();

        assert_eq!(report.len(), 1);
        let conflict = &report.conflicts()[0];
        assert_eq!(
            conflict.kind(),
            FeatureConflictKind::RemovedFeatureReferencedByLiveKbuild
        );
        assert_eq!(conflict.feature().as_str(), "bluetooth");
        assert_eq!(
            conflict.subject().as_str(),
            "kbuild:drivers/bluetooth/btusb.o"
        );
        assert_eq!(
            conflict.stable_key(),
            "removed_feature_referenced_by_live_kbuild:bluetooth:kbuild:drivers/bluetooth/btusb.o"
        );
        assert_eq!(
            conflict.summary(),
            "removed feature is still referenced by live kbuild object 'net/netfilter/core.o'"
        );
        assert_eq!(
            conflict.suggested_action(),
            "remove the 'net/netfilter/core.o' kbuild reference or preserve the removed feature"
        );
    }

    #[test]
    fn kbuild_reference_facts_are_stable_and_validate_endpoints() {
        let reference =
            FeatureKbuildReference::from_names("net/netfilter/core.o", "drivers/bluetooth/btusb.o")
                .unwrap();

        assert_eq!(reference.referencer().as_str(), "net/netfilter/core.o");
        assert_eq!(reference.referenced().as_str(), "drivers/bluetooth/btusb.o");
        assert_eq!(
            reference.stable_key(),
            "kbuild_ref:net/netfilter/core.o->drivers/bluetooth/btusb.o"
        );

        let self_reference = FeatureKbuildReference::from_names(
            "drivers/bluetooth/btusb.o",
            "drivers/bluetooth/btusb.o",
        )
        .unwrap_err();
        assert!(
            format!("{self_reference:#}").contains("kbuild reference endpoints must be distinct")
        );
    }

    #[test]
    fn reports_removed_feature_exported_symbol_consumed_by_live_feature() {
        let graph = FeatureGraph::new([
            FeatureIntent::from_config(
                FeatureIntentAction::Remove,
                "bluetooth",
                &FeatureIntentConfig {
                    exported_symbols: vec![String::from("bt_sock_register")],
                    ..FeatureIntentConfig::default()
                },
            )
            .unwrap(),
            FeatureIntent::from_config(
                FeatureIntentAction::Preserve,
                "netfilter",
                &FeatureIntentConfig {
                    roots: vec![String::from("net/netfilter")],
                    ..FeatureIntentConfig::default()
                },
            )
            .unwrap(),
            FeatureIntent::from_config(
                FeatureIntentAction::Remove,
                "wifi",
                &FeatureIntentConfig {
                    exported_symbols: vec![String::from("wifi_debugfs_init")],
                    ..FeatureIntentConfig::default()
                },
            )
            .unwrap(),
        ])
        .unwrap();

        let report = FeatureConflictReport::from_graph_and_exported_symbol_consumers(
            &graph,
            [
                FeatureExportedSymbolConsumer::from_names("netfilter", "bt_sock_register").unwrap(),
                FeatureExportedSymbolConsumer::from_names("bluetooth", "wifi_debugfs_init")
                    .unwrap(),
                FeatureExportedSymbolConsumer::from_names("netfilter", "unknown_api").unwrap(),
            ],
        )
        .unwrap();

        assert_eq!(report.len(), 1);
        let conflict = &report.conflicts()[0];
        assert_eq!(
            conflict.kind(),
            FeatureConflictKind::RemovedFeatureExportsConsumedSymbol
        );
        assert_eq!(conflict.feature().as_str(), "bluetooth");
        assert_eq!(conflict.subject().as_str(), "symbol:bt_sock_register");
        assert_eq!(
            conflict.stable_key(),
            "removed_feature_exports_consumed_symbol:bluetooth:symbol:bt_sock_register"
        );
        assert_eq!(
            conflict.summary(),
            "removed feature exports symbol 'bt_sock_register' consumed by live code"
        );
        assert_eq!(
            conflict.suggested_action(),
            "remove live consumers of 'bt_sock_register' or preserve feature 'bluetooth'"
        );
    }

    #[test]
    fn exported_symbol_consumer_facts_are_stable() {
        let consumer =
            FeatureExportedSymbolConsumer::from_names("netfilter", "bt_sock_register").unwrap();

        assert_eq!(consumer.consumer().as_str(), "netfilter");
        assert_eq!(consumer.symbol().as_str(), "bt_sock_register");
        assert_eq!(
            consumer.stable_key(),
            "exported_symbol_consumer:netfilter->bt_sock_register"
        );

        let invalid =
            FeatureExportedSymbolConsumer::from_names("netfilter", "not-a-symbol").unwrap_err();
        assert!(format!("{invalid:#}").contains("exported symbol contains invalid characters"));
    }

    #[test]
    fn reports_removed_feature_device_id_referenced_by_live_table() {
        let graph = FeatureGraph::new([
            FeatureIntent::from_config(
                FeatureIntentAction::Remove,
                "bluetooth",
                &FeatureIntentConfig {
                    pci_ids: vec![String::from("8086:1572")],
                    usb_ids: vec![String::from("0BDA:8153")],
                    ..FeatureIntentConfig::default()
                },
            )
            .unwrap(),
            FeatureIntent::from_config(
                FeatureIntentAction::Remove,
                "wifi",
                &FeatureIntentConfig {
                    acpi_ids: vec![String::from("PNP0C09")],
                    device_compatibles: vec![String::from("vendor,removed-device")],
                    ..FeatureIntentConfig::default()
                },
            )
            .unwrap(),
            FeatureIntent::from_config(
                FeatureIntentAction::Preserve,
                "netfilter",
                &FeatureIntentConfig {
                    roots: vec![String::from("net/netfilter")],
                    ..FeatureIntentConfig::default()
                },
            )
            .unwrap(),
        ])
        .unwrap();

        let report = FeatureConflictReport::from_graph_and_device_id_table_references(
            &graph,
            [
                FeatureDeviceIdTableReference::from_pci_id_names("netfilter", "8086:1572").unwrap(),
                FeatureDeviceIdTableReference::from_usb_id_names("netfilter", "0BDA:8153").unwrap(),
                FeatureDeviceIdTableReference::from_acpi_id_names("netfilter", "PNP0C09").unwrap(),
                FeatureDeviceIdTableReference::from_device_compatible_names(
                    "netfilter",
                    "vendor,removed-device",
                )
                .unwrap(),
                FeatureDeviceIdTableReference::from_pci_id_names("bluetooth", "8086:1572").unwrap(),
                FeatureDeviceIdTableReference::from_pci_id_names("netfilter", "1AF4:1000").unwrap(),
            ],
        )
        .unwrap();

        assert_eq!(report.len(), 4);
        assert_eq!(
            report
                .conflicts()
                .iter()
                .map(FeatureConflict::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "removed_feature_device_id_referenced_by_live_table:bluetooth:pci_id:8086:1572",
                "removed_feature_device_id_referenced_by_live_table:bluetooth:usb_id:0BDA:8153",
                "removed_feature_device_id_referenced_by_live_table:wifi:acpi_id:PNP0C09",
                "removed_feature_device_id_referenced_by_live_table:wifi:device_compatible:vendor,removed-device",
            ]
        );
        let conflict = &report.conflicts()[0];
        assert_eq!(
            conflict.kind(),
            FeatureConflictKind::RemovedFeatureDeviceIdReferencedByLiveTable
        );
        assert_eq!(conflict.feature().as_str(), "bluetooth");
        assert_eq!(conflict.subject().as_str(), "pci_id:8086:1572");
        assert_eq!(
            conflict.summary(),
            "removed feature owns device ID 'pci_id:8086:1572' referenced by live feature 'netfilter' table"
        );
        assert_eq!(
            conflict.suggested_action(),
            "remove 'netfilter' table reference to 'pci_id:8086:1572' or preserve feature 'bluetooth'"
        );
    }

    #[test]
    fn device_id_table_reference_facts_are_stable() {
        let reference =
            FeatureDeviceIdTableReference::from_pci_id_names("netfilter", "8086:1572").unwrap();

        assert_eq!(reference.table_owner().as_str(), "netfilter");
        assert_eq!(reference.id().kind_name(), "pci_id");
        assert_eq!(reference.id().as_str(), "8086:1572");
        assert_eq!(reference.id().stable_key(), "pci_id:8086:1572");
        assert_eq!(
            reference.stable_key(),
            "device_id_table_ref:netfilter->pci_id:8086:1572"
        );

        let compatible =
            FeatureDeviceId::from_device_compatible_name("vendor,removed-device").unwrap();
        assert_eq!(
            compatible.stable_key(),
            "device_compatible:vendor,removed-device"
        );

        let invalid =
            FeatureDeviceIdTableReference::from_usb_id_names("netfilter", "8086:zzzz").unwrap_err();
        assert!(format!("{invalid:#}").contains("USB ID must use uppercase hexadecimal digits"));
    }

    #[test]
    fn reports_removed_feature_uapi_referenced_by_live_userspace_code() {
        let graph = FeatureGraph::new([
            FeatureIntent::from_config(
                FeatureIntentAction::Remove,
                "bluetooth",
                &FeatureIntentConfig {
                    roots: vec![String::from("include/uapi/linux/bluetooth.h")],
                    ..FeatureIntentConfig::default()
                },
            )
            .unwrap(),
            FeatureIntent::from_config(
                FeatureIntentAction::Preserve,
                "netfilter",
                &FeatureIntentConfig {
                    roots: vec![String::from("net/netfilter")],
                    ..FeatureIntentConfig::default()
                },
            )
            .unwrap(),
            FeatureIntent::from_config(
                FeatureIntentAction::Remove,
                "wifi",
                &FeatureIntentConfig {
                    roots: vec![String::from("include/uapi/linux/wifi.h")],
                    ..FeatureIntentConfig::default()
                },
            )
            .unwrap(),
        ])
        .unwrap();

        let report = FeatureConflictReport::from_graph_and_userspace_uapi_references(
            &graph,
            [
                FeatureUserspaceUapiReference::from_names(
                    "netfilter",
                    "include/uapi/linux/bluetooth.h",
                )
                .unwrap(),
                FeatureUserspaceUapiReference::from_names("bluetooth", "include/uapi/linux/wifi.h")
                    .unwrap(),
                FeatureUserspaceUapiReference::from_names(
                    "netfilter",
                    "include/uapi/linux/unknown.h",
                )
                .unwrap(),
            ],
        )
        .unwrap();

        assert_eq!(report.len(), 1);
        let conflict = &report.conflicts()[0];
        assert_eq!(
            conflict.kind(),
            FeatureConflictKind::RemovedFeatureUapiReferencedByUserspaceFacingCode
        );
        assert_eq!(conflict.feature().as_str(), "bluetooth");
        assert_eq!(
            conflict.subject().as_str(),
            "uapi_header:include/uapi/linux/bluetooth.h"
        );
        assert_eq!(
            conflict.stable_key(),
            "removed_feature_uapi_referenced_by_userspace_facing_code:bluetooth:uapi_header:include/uapi/linux/bluetooth.h"
        );
        assert_eq!(
            conflict.summary(),
            "removed feature UAPI 'include/uapi/linux/bluetooth.h' is referenced by live userspace-facing code 'netfilter'"
        );
        assert_eq!(
            conflict.suggested_action(),
            "remove 'netfilter' userspace UAPI reference to 'include/uapi/linux/bluetooth.h' or preserve feature 'bluetooth'"
        );
    }

    #[test]
    fn userspace_uapi_reference_facts_are_stable() {
        let reference = FeatureUserspaceUapiReference::from_names(
            "netfilter",
            "include/uapi/linux/bluetooth.h",
        )
        .unwrap();

        assert_eq!(reference.referrer().as_str(), "netfilter");
        assert_eq!(reference.path().as_str(), "include/uapi/linux/bluetooth.h");
        assert_eq!(
            reference.stable_key(),
            "userspace_uapi_ref:netfilter->include/uapi/linux/bluetooth.h"
        );

        let invalid =
            FeatureUserspaceUapiReference::from_names("netfilter", "include/linux/private.h")
                .unwrap_err();
        assert!(format!("{invalid:#}").contains("UAPI path must be under"));
    }

    #[test]
    fn reports_removed_feature_runtime_registration_still_reachable() {
        let graph = FeatureGraph::new([
            FeatureIntent::from_config(
                FeatureIntentAction::Remove,
                "bluetooth",
                &FeatureIntentConfig {
                    runtime_registrations: vec![String::from("module_init:bt_init")],
                    remove_runtime_registrations: vec![String::from(
                        "module_platform_driver:btusb_driver",
                    )],
                    ..FeatureIntentConfig::default()
                },
            )
            .unwrap(),
            FeatureIntent::from_config(
                FeatureIntentAction::Preserve,
                "netfilter",
                &FeatureIntentConfig {
                    roots: vec![String::from("net/netfilter")],
                    ..FeatureIntentConfig::default()
                },
            )
            .unwrap(),
            FeatureIntent::from_config(
                FeatureIntentAction::Remove,
                "wifi",
                &FeatureIntentConfig {
                    runtime_registrations: vec![String::from("module_init:wifi_init")],
                    ..FeatureIntentConfig::default()
                },
            )
            .unwrap(),
        ])
        .unwrap();

        let report = FeatureConflictReport::from_graph_and_runtime_registration_reachability(
            &graph,
            [
                FeatureRuntimeRegistrationReachability::from_names(
                    "netfilter",
                    "module_init:bt_init",
                )
                .unwrap(),
                FeatureRuntimeRegistrationReachability::from_names(
                    "bluetooth",
                    "module_init:wifi_init",
                )
                .unwrap(),
                FeatureRuntimeRegistrationReachability::from_names(
                    "netfilter",
                    "module_init:unknown_init",
                )
                .unwrap(),
            ],
        )
        .unwrap();

        assert_eq!(report.len(), 1);
        let conflict = &report.conflicts()[0];
        assert_eq!(
            conflict.kind(),
            FeatureConflictKind::RemovedFeatureRuntimeRegistrationReachable
        );
        assert_eq!(conflict.feature().as_str(), "bluetooth");
        assert_eq!(
            conflict.subject().as_str(),
            "runtime_registration_surface:module_init:bt_init"
        );
        assert_eq!(
            conflict.stable_key(),
            "removed_feature_runtime_registration_reachable:bluetooth:runtime_registration_surface:module_init:bt_init"
        );
        assert_eq!(
            conflict.summary(),
            "removed feature runtime registration 'module_init:bt_init' is still reachable from live feature 'netfilter'"
        );
        assert_eq!(
            conflict.suggested_action(),
            "remove 'netfilter' runtime reachability to 'module_init:bt_init' or preserve feature 'bluetooth'"
        );
    }

    #[test]
    fn runtime_registration_reachability_facts_are_stable() {
        let reachable =
            FeatureRuntimeRegistrationReachability::from_names("netfilter", "module_init:bt_init")
                .unwrap();

        assert_eq!(reachable.reachable_from().as_str(), "netfilter");
        assert_eq!(
            reachable.runtime_registration().as_str(),
            "module_init:bt_init"
        );
        assert_eq!(
            reachable.stable_key(),
            "runtime_registration_reachability:netfilter->module_init:bt_init"
        );

        let invalid =
            FeatureRuntimeRegistrationReachability::from_names("netfilter", "module_init:1bad")
                .unwrap_err();
        assert!(format!("{invalid:#}")
            .contains("runtime registration entry point contains invalid characters"));
    }

    #[test]
    fn reports_shared_files_between_removed_and_preserved_features() {
        let graph = FeatureGraph::new([
            FeatureIntent::from_config(
                FeatureIntentAction::Remove,
                "bluetooth",
                &FeatureIntentConfig {
                    roots: vec![String::from("drivers/bluetooth")],
                    remove_paths: vec![String::from("net/bluetooth/core.c")],
                    ..FeatureIntentConfig::default()
                },
            )
            .unwrap(),
            FeatureIntent::from_config(
                FeatureIntentAction::Preserve,
                "rfkill",
                &FeatureIntentConfig {
                    roots: vec![String::from("drivers/bluetooth/btusb.c")],
                    ..FeatureIntentConfig::default()
                },
            )
            .unwrap(),
            FeatureIntent::from_config(
                FeatureIntentAction::Preserve,
                "netfilter",
                &FeatureIntentConfig {
                    roots: vec![String::from("net")],
                    ..FeatureIntentConfig::default()
                },
            )
            .unwrap(),
            FeatureIntent::from_config(
                FeatureIntentAction::Remove,
                "wifi",
                &FeatureIntentConfig {
                    roots: vec![String::from("drivers/net/wireless")],
                    ..FeatureIntentConfig::default()
                },
            )
            .unwrap(),
        ])
        .unwrap();

        let report = FeatureConflictReport::from_graph(&graph).unwrap();

        assert_eq!(report.len(), 2);
        assert_eq!(
            report
                .conflicts()
                .iter()
                .map(FeatureConflict::stable_key)
                .collect::<Vec<_>>(),
            vec![
                "shared_file_between_removed_and_preserved_features:bluetooth:path:drivers/bluetooth/btusb.c",
                "shared_file_between_removed_and_preserved_features:bluetooth:path:net/bluetooth/core.c",
            ]
        );
        let conflict = &report.conflicts()[0];
        assert_eq!(
            conflict.kind(),
            FeatureConflictKind::SharedFileBetweenRemovedAndPreservedFeatures
        );
        assert_eq!(conflict.feature().as_str(), "bluetooth");
        assert_eq!(
            conflict.subject().as_str(),
            "path:drivers/bluetooth/btusb.c"
        );
        assert_eq!(
            conflict.summary(),
            "removed feature 'bluetooth' shares path 'drivers/bluetooth/btusb.c' with preserved feature 'rfkill'"
        );
        assert_eq!(
            conflict.suggested_action(),
            "split shared path 'drivers/bluetooth/btusb.c', narrow feature roots, or preserve feature 'bluetooth'"
        );
    }

    #[test]
    fn shared_file_conflicts_use_component_boundaries() {
        let graph = FeatureGraph::new([
            FeatureIntent::from_config(
                FeatureIntentAction::Remove,
                "bluetooth",
                &FeatureIntentConfig {
                    roots: vec![String::from("drivers/bluetooth")],
                    ..FeatureIntentConfig::default()
                },
            )
            .unwrap(),
            FeatureIntent::from_config(
                FeatureIntentAction::Preserve,
                "rfkill",
                &FeatureIntentConfig {
                    roots: vec![String::from("drivers/bluetooth-extra")],
                    ..FeatureIntentConfig::default()
                },
            )
            .unwrap(),
        ])
        .unwrap();

        let report = FeatureConflictReport::from_graph(&graph).unwrap();

        assert!(report.is_empty());
    }
}
