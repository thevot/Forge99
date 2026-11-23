pub mod actor;
pub mod corpus;
pub mod level;
pub mod polys;
pub mod provenance;
pub mod tagged_property;

pub use actor::ActorRecord;
pub use corpus::{
    format_corpus_summary_markdown, scan_maps_directory_report, scan_official_maps_directory, summarize_corpus_by_mode,
    CorpusScanReport, GameModeCorpusStats, OfficialGameplayDna, OfficialLightingDna, OfficialMapDna,
    OfficialTexturePalette,
};
pub use level::{
    compile_bsp_from_world_polys, parse_compiled_umodel_bsp, CompiledUModelBsp, ReachSpecRecord,
    UModelBox, UModelBspNode, UModelBspSurf, UModelLeaf, UModelLightMapIndex, UModelVert,
    UModelZoneProperties, UnrBrush, UnrLevel, UnrReadIssue, UnrValidationReport,
};
pub use polys::{RawPoly, RawPolys};
pub use provenance::{
    classify_map, format_generator_manifest, format_generator_tag, has_generator_manifest, is_generated_author,
    is_utility_map_stem, manifest_path_for, parse_generator_tag, write_generator_manifest, GeneratorTag, MapProvenance,
    GENERATOR_MANIFEST_SUFFIX, GENERATOR_TAG_PREFIX,
};
pub use tagged_property::{
    read_tagged_properties, read_tagged_properties_with_meta, write_tagged_properties,
    write_tagged_property, write_tagged_property_with_meta, TaggedProperty, TaggedValue,
};
