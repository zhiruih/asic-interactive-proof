use std::{fs, io::{stdout, Write}};
use std::time::Instant;
pub use std::cell::RefCell;
use super::math_helper as math;
use super::math_helper::Zp;

#[derive(Copy, Clone)]
pub struct Gate {
	// Input gates
	w0: usize,
	w1: usize,
	is_add: bool,
	value: Zp,
}

impl Gate {
	fn new(w0: usize, w1: usize, is_add: bool, value: u32) -> Gate {
		Gate {
			w0: w0,
			w1: w1,
			is_add: is_add,
			value: Zp::new(value),
		}
	}
	pub fn get_wiring(&self) -> (usize, usize) {
		(self.w0, self.w1)
	}
	pub fn is_add(&self) -> bool {
		self.is_add
	}
	pub fn val(&self) -> Zp {
		self.value
	}
}

pub struct ArithCircuit {
	circuit: Vec<Vec<Gate>>,
	curr_layer: RefCell<usize>,
	pub num_bits: usize,
}

impl ArithCircuit {
	pub fn new(fname: &str) -> ArithCircuit {
		// Format:
		// each layer is one line
		// w0,w1,+ w0,w1,* ...
		// First line output gate; Last line input values only
		let mut circ = ArithCircuit {
			circuit: Vec::new(),
			curr_layer: RefCell::new(0),
			num_bits: 0,
		};
		let file: String = fs::read_to_string(fname).unwrap();
		// Parse file
		let mut max_num_gates = 0;
		for l in file.lines() {
			let mut curr_layer: Vec<Gate> = Vec::new();
			let mut layer_count = circ.curr_layer.borrow_mut();
			if l.contains(",") {
				for gates in l.split_whitespace() {
					let vals: Vec<&str> = gates.split(",").collect();
					if vals.len() != 3 {
						panic!("Wrong format at layer {}", layer_count);
					}
					if !(vals[2].eq("+") || vals[2].eq("*")) {
						panic!("Wrong format at layer {}", layer_count);
					}
					curr_layer.push(Gate::new(
						vals[0].parse().unwrap(),
						vals[1].parse().unwrap(),
						vals[2].eq("+"), 0));
				}
			} else {
				for input in l.split_whitespace() {
					curr_layer.push(Gate::new(0, 0, false, input.parse().unwrap()));
				}
			}
			if max_num_gates < curr_layer.len() {
				max_num_gates = curr_layer.len();
			}
			circ.circuit.push(curr_layer);
			*layer_count += 1;
		}
		let now = Instant::now();
		circ.evaluate_circuit();
		println!("Circuit Evaluation took {}ns", now.elapsed().as_nanos());
		circ.num_bits = (max_num_gates as f64).log2().ceil() as usize;
		*circ.curr_layer.borrow_mut() = 0;
		return circ;
	}
	fn evaluate_circuit(&mut self) {
		let circ = &mut self.circuit;
		for i in (0..circ.len() - 1).rev() {
			for j in 0..circ[i].len() {
				let w0 = circ[i+1][circ[i][j].w0].value;
				let w1 = circ[i+1][circ[i][j].w1].value;
				if circ[i][j].is_add {
					circ[i][j].value = w0 + w1;
				} else {
					circ[i][j].value = w0 * w1;
				}
			}
		}
	}
	pub fn print_circuit(&self) {
		let circ = &self.circuit;
		let mut stdout_lock = stdout().lock();
		for layer in circ {
			for gate in layer {
				write!(stdout_lock, "{},{},{} ", gate.w0, gate.w1, gate.value).unwrap();
			}
			write!(stdout_lock, "\n").unwrap();
		}
	}
	pub fn get_inputs(&self) -> &Vec<Gate> {
		&self.circuit[self.circuit.len() - 1]
	}
	pub fn set_curr_layer(&self, layer: usize) {
		*self.curr_layer.borrow_mut() = layer % (self.circuit.len());
	}
	pub fn get_last_layer(&self) -> &Vec<Gate> {
		let layer_count = *self.curr_layer.borrow();
		&self.circuit[layer_count - 1]
	}
	pub fn get_this_layer(&self) -> &Vec<Gate> {
		let layer_count = *self.curr_layer.borrow();
		&self.circuit[layer_count]
	}
	pub fn num_layers(&self) -> usize {
		self.circuit.len() - 1
	}
	pub fn next_layer(&self) {
		let layer_count = *self.curr_layer.borrow();
		self.set_curr_layer(layer_count + 1);
	}
	pub fn get_gate_val(&self, gate_lbl: usize) -> Zp {
		let layer_count = *self.curr_layer.borrow();
		self.circuit[layer_count][gate_lbl].value
	}
	pub fn get_gate_wiring(&self, gate_lbl: usize) -> (usize, usize) {
		let layer_count = *self.curr_layer.borrow();
		let gate = &self.circuit[layer_count][gate_lbl];
		(gate.w0, gate.w1)
	}
	pub fn is_gate_add(&self, gate_lbl: usize) -> bool {
		let layer_count = *self.curr_layer.borrow();
		self.circuit[layer_count - 1][gate_lbl].is_add
	}
	pub fn mle_gate_val(&self, query_gate: &[Zp]) -> Zp {
		assert_eq!(self.num_bits, query_gate.len());
		let mut val = Zp::new(0);
		for (i, &gate) in self.get_this_layer().into_iter().enumerate() {
			let orig_bits = math::into_bit_arr(i, self.num_bits);
			val += gate.val() * math::mle_interpolate(&orig_bits, query_gate);
		}
		val
	}
	pub fn mle_wiring(&self, query_gate: &[Zp], w0: &[Zp], w1: &[Zp], add: bool) -> Zp {
		assert_eq!(self.num_bits, query_gate.len());
		let mut val = Zp::new(0);
		for (i, &gate) in self.get_last_layer().into_iter().enumerate() {
			if gate.is_add() == add {
				let conn_gates = gate.get_wiring();
				let mut combined_label: Vec<Zp> = math::into_bit_arr(i, self.num_bits);
				combined_label.extend(math::into_bit_arr(conn_gates.0, self.num_bits));
				combined_label.extend(math::into_bit_arr(conn_gates.1, self.num_bits));
				let mut query_lbl: Vec<Zp> = Vec::new();
				query_lbl.extend(query_gate);
				query_lbl.extend(w0);
				query_lbl.extend(w1);
				val += math::mle_interpolate(&combined_label, &query_lbl);
			}
		}
		val
	}
}

