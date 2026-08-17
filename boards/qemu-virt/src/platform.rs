use fdt::{Fdt, Reg, Token, first_reg, first_string, stdout_path as parse_stdout_path, u32_value};

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
    stdout_path: &'a str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Error {
    Parser(fdt::Error),
    MissingRootCompatible,
    UnexpectedRootCompatible,
    MissingAddressCells,
    MissingSizeCells,
    MissingMemory,
    MissingStdoutPath,
    InvalidStdoutPath,
    MissingConsoleNode,
    MissingConsoleCompatible,
    UnsupportedConsole,
    MissingConsoleReg,
    MissingInterruptController,
    MissingInterruptControllerReg,
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
            Self::MissingAddressCells => "missing root #address-cells",
            Self::MissingSizeCells => "missing root #size-cells",
            Self::MissingMemory => "missing memory range",
            Self::MissingStdoutPath => "missing /chosen/stdout-path",
            Self::InvalidStdoutPath => "invalid stdout-path",
            Self::MissingConsoleNode => "missing console node",
            Self::MissingConsoleCompatible => "missing console compatible",
            Self::UnsupportedConsole => "console is not PL011",
            Self::MissingConsoleReg => "missing console reg",
            Self::MissingInterruptController => "missing interrupt controller",
            Self::MissingInterruptControllerReg => "missing interrupt controller reg",
            Self::InvalidTreeDepth => "invalid device tree depth",
        }
    }
}

fn discover_root<'a>(fdt: &Fdt<'a>) -> Result<RootInfo<'a>, Error> {
    let mut depth = 0_usize;

    let mut compatible = None;
    let mut address_cells = None;
    let mut size_cells = None;
    let mut memory = None;
    let mut stdout_path = None;

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
                    stdout_path = Some(parse_stdout_path(value)?);
                }
            }

            Token::EndNode => {
                if depth == 2 {
                    if memory.is_none()
                        && in_memory_node
                        && memory_device_type_matches
                        && let Some(value) = memory_reg_value
                    {
                        let root_address_cells = address_cells.ok_or(Error::MissingAddressCells)?;
                        let root_size_cells = size_cells.ok_or(Error::MissingSizeCells)?;

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

    if compatible != "linux,dummy-virt" {
        return Err(Error::UnexpectedRootCompatible);
    }

    let address_cells = address_cells.ok_or(Error::MissingAddressCells)?;
    let size_cells = size_cells.ok_or(Error::MissingSizeCells)?;
    let memory = memory.ok_or(Error::MissingMemory)?;

    let stdout_path = stdout_path.ok_or(Error::MissingStdoutPath)?;

    Ok(RootInfo {
        compatible,
        address_cells,
        size_cells,
        memory,
        stdout_path,
    })
}

#[derive(Clone, Copy, Debug)]
struct PathMatcher<'a> {
    path: &'a str,
    segment_count: usize,
    matched_segments: usize,
}

impl<'a> PathMatcher<'a> {
    fn new(path: &'a str) -> Result<Self, Error> {
        let relative = path.strip_prefix('/').ok_or(Error::InvalidStdoutPath)?;

        if relative.is_empty() {
            return Err(Error::InvalidStdoutPath);
        }

        let mut segment_count = 0_usize;

        for segment in relative.split('/') {
            if segment.is_empty() {
                return Err(Error::InvalidStdoutPath);
            }

            segment_count = segment_count
                .checked_add(1)
                .ok_or(Error::InvalidTreeDepth)?;
        }

        Ok(Self {
            path,
            segment_count,
            matched_segments: 0,
        })
    }

    fn enter_node(&mut self, name: &str, parent_depth: usize) -> Result<(), Error> {
        if parent_depth == 0 {
            if !name.is_empty() {
                return Err(Error::InvalidTreeDepth);
            }

            return Ok(());
        }

        let expected_parent_depth = self
            .matched_segments
            .checked_add(1)
            .ok_or(Error::InvalidTreeDepth)?;

        if parent_depth == expected_parent_depth
            && self.segment(self.matched_segments) == Some(name)
        {
            self.matched_segments = self
                .matched_segments
                .checked_add(1)
                .ok_or(Error::InvalidTreeDepth)?;
        }

        Ok(())
    }

    fn exit_node(&mut self, current_depth: usize) -> Result<(), Error> {
        if self.matched_segments == 0 {
            return Ok(());
        }

        let matched_node_depth = self
            .matched_segments
            .checked_add(1)
            .ok_or(Error::InvalidTreeDepth)?;

        if current_depth == matched_node_depth {
            self.matched_segments = self
                .matched_segments
                .checked_sub(1)
                .ok_or(Error::InvalidTreeDepth)?;
        }

        Ok(())
    }

    fn is_current_node(&self, current_depth: usize) -> bool {
        self.matched_segments == self.segment_count
            && self.segment_count.checked_add(1) == Some(current_depth)
    }

    fn segment(&self, index: usize) -> Option<&'a str> {
        self.path.strip_prefix('/')?.split('/').nth(index)
    }
}

fn discover_console(fdt: &Fdt<'_>, root: &RootInfo<'_>) -> Result<Reg, Error> {
    let mut matcher = PathMatcher::new(root.stdout_path)?;
    let mut depth = 0_usize;

    let mut console_compatible = None;
    let mut console_reg_value = None;

    for token in fdt.tokens() {
        match token? {
            Token::BeginNode { name } => {
                matcher.enter_node(name, depth)?;

                depth = depth.checked_add(1).ok_or(Error::InvalidTreeDepth)?;

                if matcher.is_current_node(depth) {
                    console_compatible = None;
                    console_reg_value = None;
                }
            }

            Token::Property { name, value } if matcher.is_current_node(depth) => match name {
                "compatible" => {
                    console_compatible = Some(first_string(value)? == "arm,pl011");
                }

                "reg" => {
                    console_reg_value = Some(value);
                }

                _ => {}
            },

            Token::Property { .. } => {}

            Token::EndNode => {
                if matcher.is_current_node(depth) {
                    let compatible = console_compatible.ok_or(Error::MissingConsoleCompatible)?;

                    if !compatible {
                        return Err(Error::UnsupportedConsole);
                    }

                    let reg_value = console_reg_value.ok_or(Error::MissingConsoleReg)?;

                    return first_reg(reg_value, root.address_cells, root.size_cells)
                        .map_err(Error::from);
                }

                matcher.exit_node(depth)?;

                depth = depth.checked_sub(1).ok_or(Error::InvalidTreeDepth)?;
            }

            Token::Nop => {}

            Token::End => break,
        }
    }

    Err(Error::MissingConsoleNode)
}

fn discover_interrupt_controller(fdt: &Fdt<'_>, root: &RootInfo<'_>) -> Result<Reg, Error> {
    let mut depth = 0_usize;

    let mut is_interrupt_controller = false;
    let mut reg_value = None;

    for token in fdt.tokens() {
        match token? {
            Token::BeginNode { .. } => {
                depth = depth.checked_add(1).ok_or(Error::InvalidTreeDepth)?;

                if depth == 2 {
                    is_interrupt_controller = false;
                    reg_value = None;
                }
            }

            Token::Property { name, value } if depth == 2 => match name {
                "interrupt-controller" => {
                    is_interrupt_controller = true;
                }

                "reg" => {
                    reg_value = Some(value);
                }

                _ => {}
            },

            Token::Property { .. } => {}

            Token::EndNode => {
                if depth == 2 && is_interrupt_controller {
                    let reg_value = reg_value.ok_or(Error::MissingInterruptControllerReg)?;

                    return first_reg(reg_value, root.address_cells, root.size_cells)
                        .map_err(Error::from);
                }

                depth = depth.checked_sub(1).ok_or(Error::InvalidTreeDepth)?;
            }

            Token::Nop => {}

            Token::End => break,
        }
    }

    Err(Error::MissingInterruptController)
}

pub(crate) fn discover<'a>(fdt: &Fdt<'a>) -> Result<PlatformInfo<'a>, Error> {
    let root = discover_root(fdt)?;
    let uart = discover_console(fdt, &root)?;
    let interrupt_controller = discover_interrupt_controller(fdt, &root)?;

    Ok(PlatformInfo {
        compatible: root.compatible,
        memory: root.memory,
        uart,
        interrupt_controller,
    })
}
