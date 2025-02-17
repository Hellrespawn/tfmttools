use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::Result;

#[derive(Debug, PartialEq)]
pub enum FileTreeNodeType {
    Directory(Vec<FileTreeNode>),
    File,
}

#[derive(Debug, PartialEq)]
pub struct FileTreeNode {
    name: String,
    node_type: FileTreeNodeType,
}

impl FileTreeNode {
    pub fn from_path(path: &Utf8Path) -> Result<Self> {
        let name = path.file_name().unwrap().to_owned();

        Ok(if path.is_file() {
            FileTreeNode { name, node_type: FileTreeNodeType::File }
        } else {
            let children = fs_err::read_dir(path)
                .expect("Unable to read temp_dir")
                .flatten()
                .map(|entry| {
                    Self::from_path(
                        &Utf8PathBuf::from_path_buf(entry.path().to_path_buf())
                            .unwrap(),
                    )
                })
                .collect::<Result<_>>()?;

            FileTreeNode {
                name,
                node_type: FileTreeNodeType::Directory(children),
            }
        })
    }

    fn write_prefix(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        depth: usize,
    ) -> std::fmt::Result {
        const FIRST_PREFIX: &str = "  ";
        const MIDDLE_PREFIX: &str = "│ ";
        const LAST_PREFIX: &str = "├ ";

        match depth {
            0 => (),
            1 => {
                write!(f, "{}", LAST_PREFIX)?;
            },
            2 => {
                write!(f, "{}{}", FIRST_PREFIX, LAST_PREFIX)?;
            },
            n => {
                write!(
                    f,
                    "{}{}{}",
                    FIRST_PREFIX,
                    MIDDLE_PREFIX.repeat(n - 2),
                    LAST_PREFIX
                )?;
            },
        }

        Ok(())
    }

    fn write_node(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        depth: usize,
    ) -> std::fmt::Result {
        self.write_prefix(f, depth)?;

        match &self.node_type {
            FileTreeNodeType::Directory(file_tree_nodes) => {
                writeln!(f, "{}", self.name)?;
                for node in file_tree_nodes {
                    node.write_node(f, depth + 1)?;
                }
            },
            FileTreeNodeType::File => writeln!(f, "{}", self.name)?,
        }

        Ok(())
    }
}

impl std::fmt::Display for FileTreeNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.write_node(f, 0)
    }
}
