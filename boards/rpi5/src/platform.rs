use fdt::{Fdt, Range, Reg, Token, first_range, first_reg, first_string, u32_value};

const EXPECTED_ROOT_COMPATIBLE: &str = "raspberrypi,5-model-b";
const EXPECTED_SOC_COMPATIBLE: &str = "simple-bus";
const EXPECTED_UART_COMPATIBLE: &str = "arm,pl011";
const EXPECTED_GIC_COMPATIBLE: &str = "arm,gic-400";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PlatformInfo<'a> {
    pub(crate) compatible: &'a str,
    pub(crate) memory: Reg,
    pub(crate) uart: Reg,
    pub(crate) interrupt_controller: Reg,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RootInfo<'a> {
    compatible: &'a str,
    address_cells: u32,
    size_cells: u32,
    memory: Reg,
    stdout_selector: &'a str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Error {
    Parser(fdt::Error),
    MissingRootCompatible,
    UnexpectedRootCompatible,
    MissingRootAddressCells,
    MissingRootSizeCells,
    MissingMemory,
    MissingStdoutPath,
    InvalidStdoutSelector,
    MissingStdoutAlias,
    InvalidStdoutAlias,
    UnsupportedConsoleTopology,
    MissingSoc,
    MissingSocCompatible,
    UnsupportedSoc,
    MissingSocAddressCells,
    MissingSocSizeCells,
    MissingSocRanges,
    MissingConsoleNode,
    MissingConsoleCompatible,
    UnsupportedConsole,
    MissingConsoleReg,
    MissingInterruptController,
    MissingInterruptControllerReg,
    AddressOutsideBusRange,
    AddressTranslationOverflow,
    InvalidTreeDepth,
}

impl From<fdt::Error> for Error {
    fn from(error: fdt::Error) -> Self {
        Self::Parser(error)
    }
}

impl Error {
    pub(crate) fn message(self) -> &'static str {
        match self {
            Self::Parser(error) => {
                let _ = error;
                "invalid FDT data"
            }

            Self::MissingRootCompatible => "missing root compatible",
            Self::UnexpectedRootCompatible => "unexpected root compatible",
            Self::MissingRootAddressCells => "missing root #address-cells",
            Self::MissingRootSizeCells => "missing root #size-cells",
            Self::MissingMemory => "missing memory range",
            Self::MissingStdoutPath => "missing /chosen/stdout-path",
            Self::InvalidStdoutSelector => "invalid stdout-path selector",
            Self::MissingStdoutAlias => "missing stdout-path alias",
            Self::InvalidStdoutAlias => "invalid stdout-path alias",
            Self::UnsupportedConsoleTopology => "unsupported console topology",
            Self::MissingSoc => "missing console parent bus",
            Self::MissingSocCompatible => "missing soc compatible",
            Self::UnsupportedSoc => "console parent is not simple-bus",
            Self::MissingSocAddressCells => "missing soc #address-cells",
            Self::MissingSocSizeCells => "missing soc #size-cells",
            Self::MissingSocRanges => "missing soc ranges",
            Self::MissingConsoleNode => "missing console node",
            Self::MissingConsoleCompatible => "missing console compatible",
            Self::UnsupportedConsole => "console is not PL011",
            Self::MissingConsoleReg => "missing console reg",
            Self::MissingInterruptController => "missing GIC-400",
            Self::MissingInterruptControllerReg => "missing GIC-400 reg",
            Self::AddressOutsideBusRange => "device address is outside soc range",
            Self::AddressTranslationOverflow => "translated address overflow",
            Self::InvalidTreeDepth => "invalid device tree depth",
        }
    }
}

fn parse_stdout_selector(value: &[u8]) -> Result<&str, Error> {
    let selector_with_options = first_string(value)?;

    let selector = selector_with_options
        .split_once(':')
        .map_or(selector_with_options, |(selector, _options)| selector);

    if selector.is_empty() {
        return Err(Error::InvalidStdoutSelector);
    }

    if selector.starts_with('/') {
        validate_absolute_path(selector)?;
    } else if selector.contains('/') {
        return Err(Error::InvalidStdoutSelector);
    }

    Ok(selector)
}

fn validate_absolute_path(path: &str) -> Result<(), Error> {
    let relative = path.strip_prefix('/').ok_or(Error::InvalidStdoutAlias)?;

    if relative.is_empty() || relative.split('/').any(|segment| segment.is_empty()) {
        return Err(Error::InvalidStdoutAlias);
    }

    Ok(())
}

fn discover_root<'a>(fdt: &Fdt<'a>) -> Result<RootInfo<'a>, Error> {
    let mut depth = 0_usize;

    let mut compatible = None;
    let mut address_cells = None;
    let mut size_cells = None;
    let mut memory = None;
    let mut stdout_selector = None;

    let mut in_memory_node = false;
    let mut memory_device_type_matches = false;
    let mut memory_reg_value = None;

    let mut in_chosen_node = false;

    for token in fdt.tokens() {
        match token? {
            Token::BeginNode { name } => {
                depth = depth.checked_add(1).ok_or(Error::InvalidTreeDepth)?;

                if depth == 2 {
                    in_memory_node = name.starts_with("memory@");
                    memory_device_type_matches = false;
                    memory_reg_value = None;

                    in_chosen_node = name == "chosen";
                }
            }

            Token::Property { name, value } => {
                if depth == 1 {
                    match name {
                        "compatible" if compatible.is_none() => {
                            compatible = Some(first_string(value)?);
                        }

                        "#address-cells" if address_cells.is_none() => {
                            address_cells = Some(u32_value(value)?);
                        }

                        "#size-cells" if size_cells.is_none() => {
                            size_cells = Some(u32_value(value)?);
                        }

                        _ => {}
                    }
                }

                if depth == 2 && in_memory_node {
                    match name {
                        "device_type" => {
                            memory_device_type_matches = first_string(value)? == "memory";
                        }

                        "reg" => {
                            memory_reg_value = Some(value);
                        }

                        _ => {}
                    }
                }

                if depth == 2 && in_chosen_node && name == "stdout-path" {
                    stdout_selector = Some(parse_stdout_selector(value)?);
                }
            }

            Token::EndNode => {
                if depth == 2 {
                    if memory.is_none()
                        && in_memory_node
                        && memory_device_type_matches
                        && let Some(value) = memory_reg_value
                    {
                        let root_address_cells =
                            address_cells.ok_or(Error::MissingRootAddressCells)?;

                        let root_size_cells = size_cells.ok_or(Error::MissingRootSizeCells)?;

                        memory = Some(first_reg(value, root_address_cells, root_size_cells)?);
                    }

                    in_memory_node = false;
                    memory_device_type_matches = false;
                    memory_reg_value = None;
                    in_chosen_node = false;
                }

                depth = depth.checked_sub(1).ok_or(Error::InvalidTreeDepth)?;
            }

            Token::Nop => {}

            Token::End => break,
        }
    }

    let compatible = compatible.ok_or(Error::MissingRootCompatible)?;

    if compatible != EXPECTED_ROOT_COMPATIBLE {
        return Err(Error::UnexpectedRootCompatible);
    }

    Ok(RootInfo {
        compatible,
        address_cells: address_cells.ok_or(Error::MissingRootAddressCells)?,
        size_cells: size_cells.ok_or(Error::MissingRootSizeCells)?,
        memory: memory.ok_or(Error::MissingMemory)?,
        stdout_selector: stdout_selector.ok_or(Error::MissingStdoutPath)?,
    })
}

fn resolve_console_path<'a>(fdt: &Fdt<'a>, selector: &'a str) -> Result<&'a str, Error> {
    if selector.starts_with('/') {
        validate_absolute_path(selector)?;
        return Ok(selector);
    }

    let mut depth = 0_usize;
    let mut in_aliases = false;

    for token in fdt.tokens() {
        match token? {
            Token::BeginNode { name } => {
                depth = depth.checked_add(1).ok_or(Error::InvalidTreeDepth)?;

                if depth == 2 {
                    in_aliases = name == "aliases";
                }
            }

            Token::Property { name, value } if depth == 2 && in_aliases && name == selector => {
                let path = first_string(value)?;
                validate_absolute_path(path)?;
                return Ok(path);
            }

            Token::Property { .. } => {}

            Token::EndNode => {
                if depth == 2 {
                    in_aliases = false;
                }

                depth = depth.checked_sub(1).ok_or(Error::InvalidTreeDepth)?;
            }

            Token::Nop => {}

            Token::End => break,
        }
    }

    Err(Error::MissingStdoutAlias)
}

fn split_direct_child_path(path: &str) -> Result<(&str, &str), Error> {
    let relative = path.strip_prefix('/').ok_or(Error::InvalidStdoutAlias)?;

    let mut segments = relative.split('/');

    let parent = segments.next().ok_or(Error::UnsupportedConsoleTopology)?;

    let child = segments.next().ok_or(Error::UnsupportedConsoleTopology)?;

    if parent.is_empty() || child.is_empty() || segments.next().is_some() {
        return Err(Error::UnsupportedConsoleTopology);
    }

    Ok((parent, child))
}

fn translate_reg(reg: Reg, range: Range) -> Result<Reg, Error> {
    let range_end = range
        .child_address
        .checked_add(range.size)
        .ok_or(Error::AddressTranslationOverflow)?;

    let reg_end = reg
        .address
        .checked_add(reg.size)
        .ok_or(Error::AddressTranslationOverflow)?;

    if reg.address < range.child_address || reg_end > range_end {
        return Err(Error::AddressOutsideBusRange);
    }

    let offset = reg
        .address
        .checked_sub(range.child_address)
        .ok_or(Error::AddressTranslationOverflow)?;

    let address = range
        .parent_address
        .checked_add(offset)
        .ok_or(Error::AddressTranslationOverflow)?;

    Ok(Reg {
        address,
        size: reg.size,
    })
}

fn discover_soc_devices(
    fdt: &Fdt<'_>,
    root: &RootInfo<'_>,
    console_path: &str,
) -> Result<(Reg, Reg), Error> {
    let (soc_node_name, console_node_name) = split_direct_child_path(console_path)?;

    let mut depth = 0_usize;
    let mut in_soc = false;
    let mut soc_found = false;

    let mut soc_compatible = None;
    let mut soc_address_cells = None;
    let mut soc_size_cells = None;
    let mut soc_ranges_value = None;

    let mut current_is_console = false;
    let mut console_seen = false;
    let mut current_console_compatible = None;
    let mut current_console_reg_value = None;
    let mut selected_console_compatible = None;
    let mut selected_console_reg_value = None;

    let mut current_is_interrupt_controller = false;
    let mut current_gic_compatible = false;
    let mut current_gic_reg_value = None;
    let mut gic_seen = false;
    let mut selected_gic_reg_value = None;

    for token in fdt.tokens() {
        match token? {
            Token::BeginNode { name } => {
                depth = depth.checked_add(1).ok_or(Error::InvalidTreeDepth)?;

                if depth == 2 {
                    in_soc = name == soc_node_name;

                    if in_soc {
                        soc_found = true;
                        soc_compatible = None;
                        soc_address_cells = None;
                        soc_size_cells = None;
                        soc_ranges_value = None;
                    }
                }

                if depth == 3 && in_soc {
                    current_is_console = name == console_node_name;

                    if current_is_console {
                        console_seen = true;
                    }

                    current_console_compatible = None;
                    current_console_reg_value = None;

                    current_is_interrupt_controller = false;
                    current_gic_compatible = false;
                    current_gic_reg_value = None;
                }
            }

            Token::Property { name, value } => {
                if depth == 2 && in_soc {
                    match name {
                        "compatible" => {
                            soc_compatible = Some(first_string(value)? == EXPECTED_SOC_COMPATIBLE);
                        }

                        "#address-cells" => {
                            soc_address_cells = Some(u32_value(value)?);
                        }

                        "#size-cells" => {
                            soc_size_cells = Some(u32_value(value)?);
                        }

                        "ranges" => {
                            soc_ranges_value = Some(value);
                        }

                        _ => {}
                    }
                }

                if depth == 3 && in_soc {
                    if current_is_console {
                        match name {
                            "compatible" => {
                                current_console_compatible =
                                    Some(first_string(value)? == EXPECTED_UART_COMPATIBLE);
                            }

                            "reg" => {
                                current_console_reg_value = Some(value);
                            }

                            _ => {}
                        }
                    }

                    match name {
                        "interrupt-controller" => {
                            current_is_interrupt_controller = true;
                        }

                        "compatible" => {
                            current_gic_compatible =
                                first_string(value)? == EXPECTED_GIC_COMPATIBLE;
                        }

                        "reg" => {
                            current_gic_reg_value = Some(value);
                        }

                        _ => {}
                    }
                }
            }

            Token::EndNode => {
                if depth == 3 && in_soc {
                    if current_is_console {
                        selected_console_compatible = current_console_compatible;

                        selected_console_reg_value = current_console_reg_value;
                    }

                    if current_is_interrupt_controller && current_gic_compatible {
                        gic_seen = true;

                        if selected_gic_reg_value.is_none() {
                            selected_gic_reg_value = current_gic_reg_value;
                        }
                    }

                    current_is_console = false;
                    current_is_interrupt_controller = false;
                }

                if depth == 2 {
                    in_soc = false;
                }

                depth = depth.checked_sub(1).ok_or(Error::InvalidTreeDepth)?;
            }

            Token::Nop => {}

            Token::End => break,
        }
    }

    if !soc_found {
        return Err(Error::MissingSoc);
    }

    let soc_compatible = soc_compatible.ok_or(Error::MissingSocCompatible)?;

    if !soc_compatible {
        return Err(Error::UnsupportedSoc);
    }

    let soc_address_cells = soc_address_cells.ok_or(Error::MissingSocAddressCells)?;

    let soc_size_cells = soc_size_cells.ok_or(Error::MissingSocSizeCells)?;

    let ranges_value = soc_ranges_value.ok_or(Error::MissingSocRanges)?;

    let range = first_range(
        ranges_value,
        soc_address_cells,
        root.address_cells,
        soc_size_cells,
    )?;

    if !console_seen {
        return Err(Error::MissingConsoleNode);
    }

    let console_compatible = selected_console_compatible.ok_or(Error::MissingConsoleCompatible)?;

    if !console_compatible {
        return Err(Error::UnsupportedConsole);
    }

    let console_reg_value = selected_console_reg_value.ok_or(Error::MissingConsoleReg)?;

    let console_bus_reg = first_reg(console_reg_value, soc_address_cells, soc_size_cells)?;

    if !gic_seen {
        return Err(Error::MissingInterruptController);
    }

    let gic_reg_value = selected_gic_reg_value.ok_or(Error::MissingInterruptControllerReg)?;

    let gic_bus_reg = first_reg(gic_reg_value, soc_address_cells, soc_size_cells)?;

    Ok((
        translate_reg(console_bus_reg, range)?,
        translate_reg(gic_bus_reg, range)?,
    ))
}

pub(crate) fn discover<'a>(fdt: &Fdt<'a>) -> Result<PlatformInfo<'a>, Error> {
    let root = discover_root(fdt)?;

    let console_path = resolve_console_path(fdt, root.stdout_selector)?;

    let (uart, interrupt_controller) = discover_soc_devices(fdt, &root, console_path)?;

    Ok(PlatformInfo {
        compatible: root.compatible,
        memory: root.memory,
        uart,
        interrupt_controller,
    })
}
