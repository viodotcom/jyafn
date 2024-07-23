mod optimize;
mod qbe_app;

use std::{
    io::Write,
    process::{Command, Stdio},
};
use tempfile::NamedTempFile;

use crate::Function;

use super::{Error, Graph, Node};

impl Graph {
    fn find_illegal(&self) -> Option<&Node> {
        self.nodes
            .iter()
            .find(|node| node.op.is_illegal(&node.args))
    }

    pub fn render(&self) -> qbe::Module<'static> {
        let mut module = qbe::Module::new();
        self.clone().do_render(&mut module, "run");
        module
    }

    fn do_render(&mut self, module: &mut qbe::Module<'static>, namespace: &str) {
        // Rendering main:
        let main = module.add_function(qbe::Function::new(
            qbe::Linkage::public(),
            namespace.to_string(),
            vec![
                (qbe::Type::Long, qbe::Value::Temporary("in".to_string())),
                (qbe::Type::Long, qbe::Value::Temporary("out".to_string())),
            ],
            Some(qbe::Type::Long),
        ));
        main.add_block("start".to_string());

        for (id, input) in self.inputs.iter().enumerate() {
            main.assign_instr(
                qbe::Value::Temporary(format!("i{id}")),
                input.render(),
                qbe::Instr::Load(input.render(), qbe::Value::Temporary("in".to_string())),
            );
            main.assign_instr(
                qbe::Value::Temporary("in".to_string()),
                qbe::Type::Long,
                qbe::Instr::Add(
                    qbe::Value::Const(input.size() as u64),
                    qbe::Value::Temporary("in".to_string()),
                ),
            );
        }

        // This is the old naive implementation, kept here in case you need a quick
        // rollback...
        // // Supposes that the nodes were already declared in topological order:
        // for (id, (node, is_reachable)) in self.nodes.iter().zip(reachable).enumerate() {
        //     if is_reachable {
        //         node.op
        //             .render_into(&self, Ref::Node(id).render(), &node.args, main)
        //     }
        // }

        // This is the fancier implementation, that passes the right nodes to the inside
        // of conditionals.
        optimize::const_eval(self);
        let reachable = optimize::find_reachable(&self.outputs, &self.nodes);
        optimize::Statements::build(&self.nodes).render_into(self, &reachable, main, namespace);

        for output in &self.outputs {
            main.add_instr(qbe::Instr::Store(
                self.type_of(*output).render(),
                qbe::Value::Temporary("out".to_string()),
                output.render(),
            ));
            main.assign_instr(
                qbe::Value::Temporary("out".to_string()),
                qbe::Type::Long,
                qbe::Instr::Add(
                    qbe::Value::Const(self.type_of(*output).size() as u64),
                    qbe::Value::Temporary("out".to_string()),
                ),
            );
        }

        main.add_instr(qbe::Instr::Ret(Some(qbe::Value::Const(0))));

        // Render error messages:
        for (error_id, error) in self.errors.iter().enumerate() {
            module.add_data(qbe::DataDef::new(
                qbe::Linkage::private(),
                format!("{namespace}.error.{error_id}"),
                None,
                vec![
                    (qbe::Type::Byte, qbe::DataItem::Str(error.to_string())),
                    (qbe::Type::Byte, qbe::DataItem::Const(0)),
                ],
            ));
        }

        // Rendering mapping access functions:
        for (name, mapping) in &self.mappings {
            module.add_function(mapping.render(format!("{namespace}.mapping.{name}")));
        }

        // Render sub-graphs:
        for (i, subgraph) in self.subgraphs.iter_mut().enumerate() {
            subgraph.do_render(module, &format!("{namespace}.graph.{i}"))
        }
    }

    // fn render(&self) -> &'static str {
    //     r#"
    //     export function l $run(l %in, l %out) {
    //         @start
    //                 %i0 =d loadd %in
    //                 %in =l add 8, %in
    //                 %i1 =d loadd %in
    //                 %in =l add 8, %in
    //                 %n0 =d add %i0, %i1
    //                 %c0 =d loadd $c0
    //                 %n1 =d add %n0, %c0
    //                 stored %n1, %out
    //                 %out =l add 8, %out
    //                 ret 0
    //         }
    //         data $c0 = { d d_1 }
    //     "#
    // }

    pub fn render_assembly(&self) -> Result<String, Error> {
        let rendered = self.render();
        create_assembly(rendered)
    }

    pub fn compile(&self) -> Result<Function, Error> {
        let mut graph = self.clone();
        let mut module = qbe::Module::new();
        graph.do_render(&mut module, "run");

        if let Some(node) = graph.find_illegal() {
            return Err(Error::IllegalInstruction(format!("{node:?}")));
        }

        let assembly = create_assembly(module)?;
        let unlinked = assemble(&assembly)?;
        let shared_object = link(&unlinked)?;

        Function::init(self.clone(), shared_object)
    }
}

fn create_assembly<R>(rendered: R) -> Result<String, Error>
where
    R: std::fmt::Display,
{
    let mut qbe = Command::new(qbe_app::get_qbe()?)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let mut stdin = qbe.stdin.take().expect("qbe stdin stream not captured");
    stdin.write_all(rendered.to_string().as_bytes())?;
    drop(stdin);

    let qbe_output = qbe.wait_with_output()?;
    if !qbe_output.status.success() {
        return Err(Error::Qbe {
            status: qbe_output.status,
            err: String::from_utf8_lossy(&qbe_output.stderr).to_string(),
        });
    }

    Ok(String::from_utf8_lossy(&qbe_output.stdout).to_string())
}

#[cfg(target_os = "macos")]
fn assemble(assembly: &str) -> Result<Vec<u8>, Error> {
    let mut r#as = Command::new("as")
        .args(["-o", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let mut stdin = r#as.stdin.take().expect("qbe stdin stream not captured");
    stdin.write_all(assembly.as_bytes())?;
    drop(stdin);

    let as_output = r#as.wait_with_output()?;
    if !as_output.status.success() {
        return Err(Error::Assembler {
            status: as_output.status,
            err: String::from_utf8_lossy(&as_output.stderr).to_string(),
        });
    }

    Ok(as_output.stdout)
}

#[cfg(target_os = "linux")]
fn assemble(assembly: &str) -> Result<Vec<u8>, Error> {
    let tempdir = tempfile::tempdir()?;
    let output = tempdir.path().join("main.o");

    let mut r#as = Command::new("as")
        .arg("-o")
        .arg(&output)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;

    let mut stdin = r#as.stdin.take().expect("qbe stdin stream not captured");
    stdin.write_all(assembly.as_bytes())?;
    drop(stdin);

    let as_output = r#as.wait_with_output()?;
    if !as_output.status.success() {
        return Err(Error::Assembler {
            status: as_output.status,
            err: String::from_utf8_lossy(&as_output.stderr).to_string(),
        });
    }

    Ok(std::fs::read(output)?)
}

#[cfg(target_os = "macos")]
fn link(unlinked: &[u8]) -> Result<NamedTempFile, Error> {
    let tempdir = tempfile::tempdir()?;
    let input = tempdir.path().join("main.o");
    let output = NamedTempFile::new()?;
    std::fs::write(&input, unlinked)?;

    let linker = Command::new("ld")
        .arg("-demangle")
        .arg("-dylib")
        .arg("-L")
        .arg("/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk/usr/lib")
        .arg("-lSystem")
        .arg("-o")
        .arg(output.path())
        .arg(input)
        .arg("-lSystem")
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .output()?;
    if !linker.status.success() {
        return Err(Error::Linker {
            status: linker.status,
            err: String::from_utf8_lossy(&linker.stderr).to_string(),
        });
    }

    Ok(output)
}

#[cfg(target_os = "linux")]
fn link(unlinked: &[u8]) -> Result<NamedTempFile, Error> {
    let tempdir = tempfile::tempdir()?;
    let input = tempdir.path().join("main.o");
    let output = NamedTempFile::new()?;
    std::fs::write(&input, unlinked)?;

    let linker = Command::new("ld")
        .arg("-shared")
        .arg(input)
        .arg("-o")
        .arg(output.path())
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .output()?;
    if !linker.status.success() {
        return Err(Error::Linker {
            status: linker.status,
            err: String::from_utf8_lossy(&linker.stderr).to_string(),
        });
    }

    Ok(output)
}
