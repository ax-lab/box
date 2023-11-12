use super::*;

#[derive(Default)]
pub(crate) struct PendingWrites<'a> {
	pub write: Vec<(Node<'a>, Expr<'a>, Span)>,
	pub index: Vec<NodeList<'a>>,
	pub splice: Vec<NodeSplice<'a>>,
	pub replace: Vec<NodesReplace<'a>>,
	pub temp: Vec<Node<'a>>,
}

impl<'a> Program<'a> {
	pub fn new(store: &'a Store) -> Self {
		let program = Self {
			store,
			output_code: Default::default(),
			engine: engine::Engine::new(),
			pending: Default::default(),
		};
		program
	}

	pub fn str<T: AsRef<str>>(&self, str: T) -> Str<'a> {
		self.store.str(str)
	}

	pub fn store<T>(&self, value: T) -> &'a mut T {
		self.store.add(value)
	}

	pub fn new_node(&mut self, expr: Expr<'a>, span: Span) -> Node<'a> {
		let node = self.store.alloc_node(expr, span);
		if node.key() != Key::None {
			self.engine.add_node(node);
		}
		node
	}

	pub fn output<T: IntoIterator<Item = Node<'a>>>(&mut self, nodes: T) {
		self.output_code.extend(nodes);
	}

	pub fn get_output(&self) -> &[Node<'a>] {
		&self.output_code
	}

	pub fn bind<T: Operator<'a> + 'a>(&mut self, key: Key<'a>, span: Span, op: T, prec: Precedence) {
		let op = self.store.add(op);
		self.engine.set(span, key, op, prec);
	}

	pub fn resolve(&mut self) -> Result<()> {
		let nodes = std::mem::take(&mut self.output_code);
		let list = self.new_list(nodes);
		while let Some(next) = self.engine.shift() {
			let op = *next.value();
			let key = *next.key();
			let range = *next.range();
			let nodes = next.into_nodes();
			op.execute(self, key, nodes, range)?;
			self.process_writes();
		}

		self.output_code = self.remove_nodes(list, ..);
		self.process_writes();

		Ok(())
	}

	pub fn compile(&self) -> Result<Vec<Code<'a>>> {
		self.check_unbound(|s| eprint!("{s}"))?;
		let mut code = Vec::new();
		for it in self.output_code.iter() {
			let it = it.expr().compile()?;
			code.push(it);
		}
		Ok(code)
	}

	pub fn run(&self, rt: &mut Runtime<'a>) -> Result<Value<'a>> {
		let code = self.compile()?;
		let mut value = Value::Unit;
		for it in code {
			value = rt.execute(&it)?;
		}
		Ok(value)
	}

	fn check_unbound<T: FnMut(&str)>(&self, mut output_error: T) -> Result<()> {
		if let Some(unbound) = self.engine.get_unbound() {
			output_error("\nThe following nodes have not been resolved:\n");
			for (key, nodes) in unbound {
				output_error(&format!("\n=> {key:?}:\n\n"));
				for node in nodes {
					output_error(&format!("- {node:?}\n"));
				}
			}
			output_error("\n");

			Err("compiling program: some nodes were not resolved")?;
		}
		Ok(())
	}
}
