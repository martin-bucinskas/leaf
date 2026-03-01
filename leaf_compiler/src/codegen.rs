use crate::ast::*;
use std::collections::HashMap;

pub struct CodeGenerator {
    asm: String,
    label_count: usize,
    locals: HashMap<String, i64>, // name -> stack offset (relative to r15)
    stack_offset: i64,
    current_function: Option<String>,
}

impl CodeGenerator {
    pub fn new() -> Self {
        Self {
            asm: String::new(),
            label_count: 0,
            locals: HashMap::new(),
            stack_offset: 0,
            current_function: None,
        }
    }

    pub fn generate(&mut self, program: &Program) -> String {
        self.asm.push_str(".data\n");
        for global in &program.globals {
            self.generate_global(global);
        }

        self.asm.push_str("\n.text\n");
        for function in &program.functions {
            self.generate_function(function);
        }

        self.asm.clone()
    }

    fn generate_global(&mut self, global: &GlobalVar) {
        let val = match &global.value {
            Expression::Literal(Literal::Int(n)) => n.to_string(),
            _ => "0".to_string(), // Simplified: globals must be constant ints for now
        };
        self.asm.push_str(&format!("{}: .word {}\n", global.name, val));
    }

    fn generate_function(&mut self, function: &Function) {
        self.asm.push_str(&format!("{}:\n", function.name));
        self.locals.clear();
        self.current_function = Some(function.name.clone());
        
        // entry SP points to return address (for non-main) or end of memory (for main).
        // We track stack relative to this point.
        self.stack_offset = 0; 

        for (i, param) in function.params.iter().enumerate() {
            self.asm.push_str(&format!("    PUSH r{}\n", i + 1));
            self.stack_offset -= 8;
            self.locals.insert(param.name.clone(), self.stack_offset);
        }

        self.generate_block(&function.body);

        // Function epilogue
        if function.name == "main" {
            self.asm.push_str("    MOVI r0, 0\n");
            self.asm.push_str("    HALT\n");
        } else {
            // implicit return if body didn't return
            self.generate_stack_cleanup();
            self.asm.push_str("    RET\n");
        }
        
        self.current_function = None;
    }

    fn generate_stack_cleanup(&mut self) {
        if self.stack_offset != 0 {
            self.asm.push_str(&format!("    MOVI r31, {}\n", -self.stack_offset));
            self.asm.push_str("    ADD r15, r15, r31\n");
        }
    }

    fn generate_block(&mut self, block: &Block) {
        for stmt in &block.statements {
            self.generate_statement(stmt);
            // If the statement was a Return, subsequent statements in this block are unreachable.
            // However, we don't have a clean way to "break" here without tracking block completion.
            // For now, we continue, but the generator should ideally stop.
            if let Statement::Return(_) = stmt {
                break;
            }
        }
    }

    fn generate_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::LocalVar(local) => {
                if let Some(val) = &local.value {
                    self.generate_expression(val, 0); // result in r0
                    self.asm.push_str("    PUSH r0\n");
                } else {
                    self.asm.push_str("    MOVI r0, 0\n");
                    self.asm.push_str("    PUSH r0\n");
                }
                self.stack_offset -= 8;
                self.locals.insert(local.name.clone(), self.stack_offset);
            }
            Statement::Assignment { name, value } => {
                self.generate_expression(value, 0); // result in r0
                if let Some(offset) = self.locals.get(name) {
                    let rel_offset = offset - self.stack_offset;
                    self.asm.push_str(&format!("    MOVI r31, {}\n", rel_offset));
                    self.asm.push_str("    ADD r31, r15, r31\n");
                    self.asm.push_str("    STORE r0, [r31]\n");
                } else {
                    // Global variable
                    self.asm.push_str(&format!("    MOVI r31, {}\n", name));
                    self.asm.push_str("    STORE r0, [r31]\n");
                }
            }
            Statement::Expression(expr) => {
                self.generate_expression(expr, 0);
            }
            Statement::Return(expr) => {
                if let Some(e) = expr {
                    self.generate_expression(e, 0); // return value in r0
                }
                self.generate_stack_cleanup();
                if let Some(func_name) = &self.current_function {
                    if func_name == "main" {
                        self.asm.push_str("    HALT\n");
                    } else {
                        self.asm.push_str("    RET\n");
                    }
                } else {
                    self.asm.push_str("    RET\n");
                }
            }
            Statement::If { condition, then_block, else_block } => {
                let else_label = self.new_label("else");
                let end_label = self.new_label("endif");

                self.generate_expression(condition, 0);
                self.asm.push_str(&format!("    JZ r0, {}\n", else_label));
                self.generate_block(then_block);
                self.asm.push_str(&format!("    JMP {}\n", end_label));
                self.asm.push_str(&format!("{}:\n", else_label));
                if let Some(eb) = else_block {
                    self.generate_block(eb);
                }
                self.asm.push_str(&format!("{}:\n", end_label));
            }
            Statement::While { condition, block } => {
                let start_label = self.new_label("while_start");
                let end_label = self.new_label("while_end");

                self.asm.push_str(&format!("{}:\n", start_label));
                self.generate_expression(condition, 0);
                self.asm.push_str(&format!("    JZ r0, {}\n", end_label));
                self.generate_block(block);
                self.asm.push_str(&format!("    JMP {}\n", start_label));
                self.asm.push_str(&format!("{}:\n", end_label));
            }
        }
    }

    fn generate_expression(&mut self, expr: &Expression, reg: usize) {
        match expr {
            Expression::FunctionCall { name, args } => {
                if name == "print" {
                    self.generate_expression(&args[0], 1); // arg in r1
                    self.asm.push_str("    MOVI r0, 2\n"); // PRINT_INT syscall
                    self.asm.push_str("    SYSCALL\n");
                } else {
                    // General function call
                    for (i, arg) in args.iter().enumerate() {
                        self.generate_expression(arg, i + 1);
                    }
                    self.asm.push_str(&format!("    CALL {}\n", name));
                    if reg != 0 {
                        self.asm.push_str(&format!("    MOV r{}, r0\n", reg));
                    }
                }
            }
            Expression::Binary { left, op, right } => {
                self.generate_expression(left, reg);
                self.asm.push_str(&format!("    PUSH r{}\n", reg));
                let old_offset = self.stack_offset;
                self.stack_offset -= 8;
                self.generate_expression(right, reg);
                self.asm.push_str("    POP r30\n"); // Left side in r30
                self.stack_offset = old_offset;
                match op {
                    BinaryOp::Add => self.asm.push_str(&format!("    ADD r{}, r30, r{}\n", reg, reg)),
                    BinaryOp::Sub => self.asm.push_str(&format!("    SUB r{}, r30, r{}\n", reg, reg)),
                    BinaryOp::Mul => self.asm.push_str(&format!("    MUL r{}, r30, r{}\n", reg, reg)),
                    BinaryOp::Div => self.asm.push_str(&format!("    DIV r{}, r30, r{}\n", reg, reg)),
                    BinaryOp::Lt => self.asm.push_str(&format!("    LT r{}, r30, r{}\n", reg, reg)),
                    BinaryOp::Gt => self.asm.push_str(&format!("    GT r{}, r30, r{}\n", reg, reg)),
                    BinaryOp::Eq => self.asm.push_str(&format!("    EQ r{}, r30, r{}\n", reg, reg)),
                    BinaryOp::Ne => {
                        self.asm.push_str(&format!("    EQ r{}, r30, r{}\n", reg, reg));
                        self.asm.push_str(&format!("    MOVI r31, 1\n"));
                        self.asm.push_str(&format!("    XOR r{}, r{}, r31\n", reg, reg));
                    }
                    BinaryOp::Le => {
                        // a <= b  <=>  !(a > b)
                        self.asm.push_str(&format!("    GT r{}, r30, r{}\n", reg, reg));
                        self.asm.push_str(&format!("    MOVI r31, 1\n"));
                        self.asm.push_str(&format!("    XOR r{}, r{}, r31\n", reg, reg));
                    }
                    BinaryOp::Ge => {
                        // a >= b  <=>  !(a < b)
                        self.asm.push_str(&format!("    LT r{}, r30, r{}\n", reg, reg));
                        self.asm.push_str(&format!("    MOVI r31, 1\n"));
                        self.asm.push_str(&format!("    XOR r{}, r{}, r31\n", reg, reg));
                    }
                    BinaryOp::And => self.asm.push_str(&format!("    AND r{}, r30, r{}\n", reg, reg)),
                    BinaryOp::Or => self.asm.push_str(&format!("    OR r{}, r30, r{}\n", reg, reg)),
                }
            }
            Expression::Literal(Literal::Int(n)) => {
                self.asm.push_str(&format!("    MOVI r{}, {}\n", reg, n));
            }
            Expression::Literal(Literal::Bool(b)) => {
                let val = if *b { 1 } else { 0 };
                self.asm.push_str(&format!("    MOVI r{}, {}\n", reg, val));
            }
            Expression::Identifier(name) => {
                if let Some(offset) = self.locals.get(name) {
                    let rel_offset = offset - self.stack_offset;
                    self.asm.push_str(&format!("    MOVI r31, {}\n", rel_offset));
                    self.asm.push_str("    ADD r31, r15, r31\n");
                    self.asm.push_str(&format!("    LOAD r{}, [r31]\n", reg));
                } else {
                    // Global variable
                    self.asm.push_str(&format!("    MOVI r31, {}\n", name));
                    self.asm.push_str(&format!("    LOADI r{}, r31\n", reg));
                }
            }
            Expression::Literal(Literal::Str(_s)) => {
                let label = self.new_label("str");
                self.asm.push_str(&format!("    MOVI r{}, {}\n", reg, label));
            }
        }
    }

    fn new_label(&mut self, prefix: &str) -> String {
        let label = format!("{}_{}", prefix, self.label_count);
        self.label_count += 1;
        label
    }
}
