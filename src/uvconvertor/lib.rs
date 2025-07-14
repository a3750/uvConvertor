use std::{
    fs::{read_to_string, File},
    path::{Path, PathBuf},
    collections::HashMap,
    io::{self, Read},
    error::Error,
    ops::Add,
};
use regex::{Captures, Regex};
use roxmltree::{Document, ExpandedName, Node};
use serde::Serialize;


#[derive(Debug)]
pub struct Convertor {
    commands: Vec<CompileCommand>,
}

#[derive(Debug, Serialize)]
struct CompileCommand {
    directory: String,
    file: String,
    arguments: Vec<String>,
}

impl Add for Convertor {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self::Output {
        self.append(rhs);
        self
    }
}

impl Convertor {
    pub fn new() -> Self {
        Self { commands: Vec::new() }
    }

    pub fn from(uvprojx: &Path, target_name: Option<&str>) -> Result<Self, Box<dyn Error>> {
        let uvdir = uvprojx.parent().ok_or("invalid UTF-8 in uvprojx path")?;
        let uvname = uvprojx.file_stem().and_then(|s| s.to_str()).ok_or("invalid UTF-8 in uvprojx path")?;

        let (outdir, outname, target_name) = Self::parse_uvprojx_file(uvprojx, target_name)?;

        let dep_path = uvdir.join(outdir.clone()).join(format!("{uvname}_{target_name}.dep"));
        let build_log_path = uvdir.join(outdir).join(format!("{outname}.build_log.htm"));

        #[cfg(target_os = "windows")]
        if !dep_path.exists() {
            println!("{dep_path:?} not exists, trying to compile...");
            Self::try_compile(uvprojx)?;
        }

        let cc = Self::find_toolchain_path(&build_log_path).and_then(|dir| {
            let mut entries = dir.read_dir().ok()?.filter_map(Result::ok);
            let cc = entries.find(|entry| {
                entry.file_name().to_str()
                    .is_some_and(|name| name.starts_with("armcc") || name.starts_with("armclang"))
            })?;
            Some(cc.path().to_string_lossy().to_string())
        });

        let commands = if let Some(cc) = cc {
            Self::parse_dep_file(&dep_path, uvdir)?
                .into_iter()
                .map(|mut command| {
                    command.arguments.insert(0, cc.clone());
                    command
                })
                .collect()
        } else {
            Self::parse_dep_file(&dep_path, uvdir)?
        };

        Ok(Self { commands })
    }

    pub fn add_arguments(&mut self, arguments: &[String]) {
        for command in &mut self.commands {
            command.arguments.extend(Vec::from(arguments));
        }
    }

    pub fn remove_arguments(&mut self, prefixes: &[String]) {
        for command in &mut self.commands {
            // 创建新参数列表
            let mut new_args = Vec::new();
            // 标记是否跳过下一个参数（当遇到需要移除的参数时）
            let mut skip_next = false;
            
            // 遍历当前命令的所有参数
            for (i, arg) in command.arguments.iter().enumerate() {
                if skip_next {
                    skip_next = false;
                    continue;
                }
                
                // 检查当前参数是否需要移除
                if prefixes.contains(arg) {
                    // 标记跳过下一个参数（如果有）
                    let next_exists = i < command.arguments.len() - 1;
                    let next_is_value = next_exists && 
                        !command.arguments[i + 1].starts_with('-');
                    
                    if next_is_value {
                        skip_next = true;
                    }
                    continue;
                }
                
                // 保留当前参数
                new_args.push(arg.clone());
            }
            
            // 更新命令参数
            command.arguments = new_args;
        }
    }

    pub fn remove_sysroot(&mut self) {
        fn contains_std_headers(path: &Path) -> bool {
            path.read_dir().is_ok_and(|mut entries| {
                entries.any(|e| {
                    e.is_ok_and(|e| {
                        e.file_name() == "stdio.h" || e.file_name() == "iostream"
                    })
                })
            })
        }

        let mut cache = HashMap::new(); // 缓存路径->是否包含头文件
        for command in &mut self.commands {
            command.arguments.retain(|argument| {
                argument.strip_prefix("-I").is_none_or(|path| {
                    *cache.entry(path.to_string()).or_insert_with(|| 
                        contains_std_headers(Path::new(path))
                    )
                })
            });
        }
    }

    pub fn replace_disk(&mut self, rep: &str) {
        let rep = Regex::new(
            r#"((?:^|[^$])(?:\$\$)*)\$(?:(DISK|disk|D|d)([^\w]|$)|\{(DISK|disk|D|d)\})"#
        ).unwrap().replace_all(rep, |caps: &Captures| {
            let prefix = &caps[1];
            let pattern = caps.get(2).or(caps.get(4)).unwrap().as_str();
            let suffix = caps.get(3).map_or("", |m| m.as_str());
            if pattern.find('D').is_some() {
                format!("{prefix}$1{suffix}")
            } else {
                format!("{prefix}$2{suffix}")
            }
        }).to_string().replace("$$", "$");

        let replace = |haystack: &str| {
            let re = Regex::new(r#"^((?:-{1,2}(?:[A-Za-z]+-)*[A-Za-z]+)?)([A-Za-z]):/"#).unwrap();
            re.replace_all(haystack, |caps: &Captures| {
                let opt = &caps[1];
                let disk = &caps[2];
                let haystack = format!("{}{}", disk.to_uppercase(), disk.to_lowercase());
                let re = Regex::new(r#"(.)(.)"#).unwrap();
                format!("{}{}/", opt, re.replace_all(&haystack, rep.clone()))
            }).to_string()
        };

        for command in &mut self.commands {
            command.file = replace(&command.file);
            for arg in &mut command.arguments {
                *arg = replace(arg);
            }
        }
    }
    
    pub fn append(&mut self, rhs: Self) {
        self.commands.extend(rhs.commands);
    }

    pub fn dump_to_json<T: io::Write>(&self, mut output: T) -> io::Result<()> {
        serde_json::to_writer_pretty(&mut output, &self.commands)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        output.write_all(b"\n")?;
        Ok(())
    }

    fn parse_uvprojx_file(
        uvprojx: &Path,
        target_name: Option<&str>,
    ) -> Result<(String, String, String), Box<dyn Error>> {
        trait FindChildByTag {
            fn find_child_by_tag<'n, 'm, N>(&self, name: N) -> Result<Node<'_, '_>, String>
            where
                N: Into<ExpandedName<'n, 'm>>;
        }

        impl<'a, 'input: 'a> FindChildByTag for Node<'a, 'input> {
            fn find_child_by_tag<'n, 'm, N>(&self, name: N) -> Result<Node<'_, '_>, String>
            where
                N: Into<ExpandedName<'n, 'm>>,
            {
                let name = name.into();
                self.children()
                    .find(|n| n.has_tag_name(name))
                    .ok_or(format!("no tag of any element named \"{name:?}\""))
            }
        }

        let text = read_to_string(uvprojx)?.replace("\\", "/");
        let doc = Document::parse(&text)?;
        let root = doc.root();
        let project = root.find_child_by_tag("Project")?;
        let targets = project.find_child_by_tag("Targets")?;

        let target = match target_name {
            Some(_) => targets
                .children()
                .find(|n| {
                    n.has_tag_name("Target") && n.children().any(|n| {
                        n.has_tag_name("TargetName") && n.text() == target_name
                    })
                })
                .ok_or("no target's name matches given name"),
            None => targets
                .children()
                .find(|n| {
                    n.has_tag_name("Target") && n.children().any(|n| n.has_tag_name("TargetName"))
                })
                .ok_or("no element named target"),
        }?;

        let name = target.find_child_by_tag("TargetName").unwrap().first_child();
        let target_name = match name {
            Some(n) if n.is_text() => n.text(),
            None => Some(""),
            _ => None,
        }.ok_or("child of element \"TargetName\" is not a text node")?.to_string();

        let option = target.find_child_by_tag("TargetOption")?;
        let common_option = option.find_child_by_tag("TargetCommonOption")?;

        let outdir = common_option
            .find_child_by_tag("OutputDirectory")?
            .text()
            .ok_or("element taged \"OutputDirectory\" has no text")?;

        let outname = common_option
            .find_child_by_tag("OutputName")?
            .text()
            .ok_or("element taged \"OutputName\" has no text")?;

        Ok((String::from(outdir), String::from(outname), target_name))
    }

    #[cfg(target_os = "windows")]
    fn try_compile(uvprojx: &Path) -> Result<(), Box<dyn Error>> {
        use std::process::Command;

        let outputs = (
            Command::new("where").arg("UV4").output()?,
            Command::new("ls").arg("C:/Keil_v5/UV4/UV4.exe").output()?,
        );

        let executable = if outputs.0.status.success() {
            Some(String::from_utf8_lossy(&outputs.0.stdout).lines().next().unwrap().to_string())
        } else if outputs.1.status.success() {
            Some(String::from("C:/Keil_v5/UV4/UV4.exe"))
        } else {
            None
        }.ok_or("compilation failed, because there is no UV4.exe in $env:PATH and \"C:/Keil_v5\" is not exists")?;

        let uvprojx_str = uvprojx
            .to_str()
            .ok_or("invalid UTF-8 character in uvprojx path")?;
        let output = Command::new(executable)
            .args(["-j", "-r", uvprojx_str])
            .output()?;

        output
            .status
            .success()
            .then_some(())
            .ok_or("compilation failed, please check your project".into())
    }

    fn parse_dep_file(dep_path: &Path, directory: &Path) -> Result<Vec<CompileCommand>, Box<dyn Error>> {
        let content = read_to_string(dep_path)?
            .replace("\\", "/")
            .replace("\r", " ")
            .replace("\n", " ");

        Regex::new(r#"F \((.*?)\)\(.*?\)\((.*?)\)"#)?
            .captures_iter(&content)
            .map(|caps| {
                let file = caps.get(1).unwrap().as_str().to_string();
                let shell_str = caps.get(2).unwrap().as_str();
                let mut args = shell_words::split(shell_str)?.into_iter().peekable();
                let arguments: Vec<_> = std::iter::from_fn(move || {
                    match args.next() {
                        Some(arg) if arg == "-I" => args.next().map(|n| format!("-I{n}")),
                        other => other,
                    }
                }).collect();
                Ok(CompileCommand {
                    directory: directory.to_string_lossy().to_string(),
                    file,
                    arguments,
                })
            })
            .collect::<Result<Vec<_>, _>>()
    }

    fn find_toolchain_path(build_log_path: &Path) -> Option<PathBuf> {
        use regex::bytes::Regex;

        let mut file = File::open(build_log_path).ok()?;
        let mut content = Vec::new();
        file.read_to_end(&mut content).ok()?;

        let toolchain_path = Regex::new(r#"Toolchain Path:\s*(.*)[\s]*"#)
            .ok()?
            .captures(&content)?
            .get(1)?
            .as_bytes();

        Some(PathBuf::from(String::from_utf8_lossy(toolchain_path).to_string()))
    }
}
