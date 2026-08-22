#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    HeaderTooSmall {
        actual: usize,
    },
    InvalidMagic {
        actual: u32,
    },
    InvalidTotalSize {
        total_size: usize,
        available: usize,
    },
    UnsupportedVersion {
        version: u32,
        last_compatible_version: u32,
    },
    IntegerOverflow,
    StructureBlockOutOfBounds,
    StringsBlockOutOfBounds,
    MemoryReservationBlockOutOfBounds,
    MisalignedStructureBlock,
    MisalignedMemoryReservationBlock,
    OverlappingBlocks,
    StructureBlockTruncated,
    UnterminatedNodeName,
    InvalidUtf8,
    TruncatedProperty,
    PropertyOutsideNode,
    PropertyNameOutOfBounds {
        offset: usize,
    },
    UnterminatedPropertyName,
    UnknownStructureToken {
        value: u32,
    },
    UnexpectedEndNode,
    UnclosedNodes {
        depth: usize,
    },
    MissingEndToken,
    TrailingStructureData,
    UnterminatedString,
    InvalidNodePath,
    InvalidU32Length {
        actual: usize,
    },
    UnsupportedCellCount {
        address_cells: u32,
        size_cells: u32,
    },
    TruncatedReg {
        required: usize,
        actual: usize,
    },
    UnsupportedRangeCellCount {
        child_address_cells: u32,
        parent_address_cells: u32,
        size_cells: u32,
    },
    TruncatedRange {
        required: usize,
        actual: usize,
    },
}
