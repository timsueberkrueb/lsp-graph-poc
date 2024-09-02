use std::fs;
use std::path::{Path, PathBuf};

use ignore::dir::Ignore;
use ignore::dir::IgnoreBuilder;

use graph::{EdgeData, Graph, NodeContents, NodeData, NodeId, Relation};

struct StackEntry {
    parent_node: NodeId,
    parent_path: PathBuf,
    parent_ignore: Ignore,
}

pub fn populate_file_structure<P: AsRef<Path>>(
    graph: &mut Graph,
    root_path: P,
) -> Result<(), anyhow::Error> {
    let root_path = root_path.as_ref().to_owned();
    let root_node = create_root_node(graph, root_path.clone())?;
    let root_ignore = IgnoreBuilder::new().hidden(true).build();
    let (root_ignore, error) = root_ignore.add_parents(root_path.clone());
    if let Some(error) = error {
        return Err(error.into());
    }
    let (root_ignore, error) = root_ignore.add_child(&root_path);
    if let Some(error) = error {
        return Err(error.into());
    }

    let mut stack = vec![StackEntry {
        parent_node: root_node,
        parent_path: root_path,
        parent_ignore: root_ignore,
    }];
    while let Some(entry) = stack.pop() {
        let StackEntry {
            parent_node,
            parent_path,
            parent_ignore,
        } = entry;

        let entries = fs::read_dir(parent_path)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if parent_ignore.is_ignored(stack.len(), &entry).is_ignore() {
                println!("Ignoring {:?}", path);
                continue;
            }
            let display_name = entry.file_name().to_string_lossy().to_string();
            let node = NodeData {
                contents: if path.is_dir() {
                    NodeContents::Folder {
                        display_name,
                        path: path.clone(),
                    }
                } else {
                    NodeContents::File {
                        display_name,
                        path: path.clone(),
                    }
                },
            };
            let node = graph.add_node(node);
            let edge = EdgeData {
                from: parent_node,
                to: node,
                relation: Relation::IsParentOf,
            };
            graph.add_edge(edge);
            if path.is_dir() {
                let (ignore, error) = parent_ignore.add_child(&path);
                if let Some(error) = error {
                    return Err(error.into());
                }
                stack.push(StackEntry {
                    parent_node: node,
                    parent_path: path,
                    parent_ignore: ignore,
                });
            }
        }
    }

    Ok(())
}

fn create_root_node(graph: &mut Graph, root_path: PathBuf) -> Result<NodeId, anyhow::Error> {
    if !root_path.is_dir() {
        anyhow::bail!("{} is not a directory", root_path.display());
    }
    let Some(dir_name) = root_path.file_name() else {
        anyhow::bail!(
            "Could not get directory name from path {}",
            root_path.display()
        );
    };
    let root_node = NodeData {
        contents: NodeContents::Folder {
            display_name: dir_name.to_string_lossy().to_string(),
            path: root_path,
        },
    };
    Ok(graph.add_node(root_node))
}
