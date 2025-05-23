use bijective_enum_map::injective_enum_map;


use prismarine_anchor_util::declare_and_pub_use;

// Enums that have their own methods get their own modules.
declare_and_pub_use! {
    actor_digest_version;
    chunk_version;
}


#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub enum BlendingVersion {
    V1_19_0,
    V1_19_0_1,
    V1_19_0_2,
    V1_19_0_3,
    V1_20_0,
    V1_20_0_1,
    V1_21_50,
    V1_21_60,
}

injective_enum_map! {
    BlendingVersion, i16,
    V1_19_0   <=> 0,
    V1_19_0_1 <=> 1,
    V1_19_0_2 <=> 2,
    V1_19_0_3 <=> 3,
    V1_20_0   <=> 4,
    V1_20_0_1 <=> 5,
    V1_21_50  <=> 6,
    V1_21_60  <=> 7,
}

// Based on rbedrock
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub enum FinalizedState {
    NeedsInstaticking,
    NeedsPopulation,
    Done,
}

injective_enum_map! {
    FinalizedState, u32,
    NeedsInstaticking <=> 0,
    NeedsPopulation   <=> 1,
    Done              <=> 2,
}

#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub enum GeneratorType {
    Old,
    Infinite,
    Flat,
}

injective_enum_map! {
    GeneratorType, i32,
    Old      <=> 0,
    Infinite <=> 1,
    Flat     <=> 2,
}

#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub enum HardcodedSpawnerType {
    NetherFortress,
    WitchHut,
    OceanMonument,
    LegacyVillageCat,
    PillagerOutpost,
    NewerLegacyVillageCat,
}

injective_enum_map! {
    HardcodedSpawnerType, u8,
    NetherFortress        <=> 1,
    WitchHut              <=> 2,
    OceanMonument         <=> 3,
    LegacyVillageCat      <=> 4,
    PillagerOutpost       <=> 5,
    NewerLegacyVillageCat <=> 6,
}







// This is here, at least for now, but should probably go with the MetaData NBT type
#[cfg_attr(feature = "derive_standard", derive(PartialEq, Eq, PartialOrd, Ord, Hash))]
#[derive(Debug, Clone, Copy)]
pub enum MetaDataType {
    LastSavedBaseGameVersion,
    OriginalBaseGameVersion,
    BiomeBaseGameVersion,
    DimensionName,
    GenerationSeed,
    GeneratorType,
    WorldGen1_18AppliedBelow0,
    Overworld1_18HeightExtended,
    BlendingVersion,
    OriginalDimensionHeightRange,
    LastSavedDimensionHeightRange,
    UnderwaterLavaLakeFixed,
    WorldGenBelowZeroFixed,
    SkullFlatteningPerformed,
}

injective_enum_map! {
    MetaDataType, u8,
    LastSavedBaseGameVersion      <=> 0,
    OriginalBaseGameVersion       <=> 1,
    BiomeBaseGameVersion          <=> 2,
    DimensionName                 <=> 3,
    GenerationSeed                <=> 4,
    GeneratorType                 <=> 5,
    WorldGen1_18AppliedBelow0     <=> 6,
    Overworld1_18HeightExtended   <=> 7,
    BlendingVersion               <=> 8,
    OriginalDimensionHeightRange  <=> 9,
    LastSavedDimensionHeightRange <=> 10,
    UnderwaterLavaLakeFixed       <=> 11,
    WorldGenBelowZeroFixed        <=> 12,
    SkullFlatteningPerformed      <=> 13,
}

injective_enum_map! {
    MetaDataType, &'static str, &str,
    LastSavedBaseGameVersion      <=> "LastSavedBaseGameVersion",
    OriginalBaseGameVersion       <=> "OriginalBaseGameVersion",
    BiomeBaseGameVersion          <=> "BiomeBaseGameVersion",
    DimensionName                 <=> "DimensionName",
    GenerationSeed                <=> "GenerationSeed",
    GeneratorType                 <=> "GeneratorType",
    WorldGen1_18AppliedBelow0     <=> "WorldGen1_18AppliedBelow0",
    Overworld1_18HeightExtended   <=> "Overworld1_18HeightExtended",
    BlendingVersion               <=> "BlendingVersion",
    OriginalDimensionHeightRange  <=> "OriginalDimensionHeightRange",
    LastSavedDimensionHeightRange <=> "LastSavedDimensionHeightRange",
    UnderwaterLavaLakeFixed       <=> "UnderwaterLavaLakeFixed",
    WorldGenBelowZeroFixed        <=> "WorldGenBelowZeroFixed",
    SkullFlatteningPerformed      <=> "SkullFlatteningPerformed",
}
