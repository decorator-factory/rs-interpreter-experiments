//! Step-by-step evaluation strategy.
//!
//! "Hmm" stands for "Highly modular microprocedures"
//! (I totally didn't make that up while writing the docstring).
//!
//! This primarily exists for when you want to execute a program
//! in small steps to limit for how long it should run, or how fast.
//!
//! ```rust
//! let expr =
//!     F!(
//!         'app F!(
//!             'app F!('lam x . F!('lam y . F![y])),
//!             'to F![420]
//!         ),
//!         'to F![69]
//!     );
//! let mut state = HmmState::new(expr, Rc::default());
//!
//! let mut step = 0;
//! let result = loop {
//!     step += 1;
//!
//!     if step >= 1000 {
//!         break Err(EvalError::Misc("Too many steps!".into()));
//!     }
//!
//!     if let Some(r) = state.step() {
//!         break r;
//!     }
//! };
//!
//! println!("Result: {:?}", result);
//! println!("Steps: {:?}", &step);
//! ```
//! Output:
//! ```
//! Result: Ok(Int(69))
//! Steps: 11
//! ```
//!
//! WARNING: this module is currently a mess!

use std::rc::Rc;

use crate::base::EvalError;

pub type Val = crate::base::Val<Builtin>;
pub type Expr = crate::base::Expr<Builtin>;
pub type Ns = crate::base::Ns<Builtin>;
pub type EvalResult = crate::base::EvalResult<Builtin>;
pub struct Builtin(Box<dyn Fn(Rc<Val>) -> Thunk>);

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
struct Marker(u64);

type Callback = Box<dyn FnOnce(Rc<Val>) -> Thunk>;

struct Entry {
    need: Rc<Expr>,
    ns: Rc<Ns>,
    callback: Callback,
    parent: Option<Marker>,
}

impl std::fmt::Debug for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Entry")
            .field("need", &self.need)
            .field("parent", &self.parent)
            .finish()
    }
}

enum Thunk {
    Done(EvalResult),
    NeedElaboration(Rc<Expr>, Rc<Ns>, Callback),
}

#[derive(Debug)]
enum Step {
    Deliver(Option<Marker>, Rc<Val>),
    Elaborate(Marker),
}

pub struct HmmState {
    last_marker: Marker,
    steps: Vec<Step>,
    blackboard: std::collections::HashMap<Marker, Entry>,
}

fn hmm_call(fun: Rc<Val>, arg: Rc<Val>) -> Thunk {
    match fun.as_ref() {
        Val::Builtin(_, f) => (f.0)(arg),
        Val::UserFn {
            argname,
            body,
            ns: closure,
        } => {
            let local_scope = closure.clone().child().update(argname.clone(), arg);
            hmm_step(body.clone(), Rc::new(local_scope))
        }
        Val::Int(_) => Thunk::Done(Err(EvalError::TypeMismatch("Cannot call integer"))),
        Val::Str(_) => Thunk::Done(Err(EvalError::TypeMismatch("Cannot call string"))),
        Val::Vec(_) => Thunk::Done(Err(EvalError::TypeMismatch("Cannot call vec"))),
    }
}

fn hmm_step(expr: Rc<Expr>, ns: Rc<Ns>) -> Thunk {
    match expr.as_ref() {
        Expr::Put(val) => Thunk::Done(Ok(val.clone())),
        Expr::Name(name) => Thunk::Done(match ns.lookup(name.clone()) {
            Some(v) => Ok(v),
            None => Err(EvalError::UndefinedName(name.as_ref().clone())),
        }),
        Expr::Lam { argname, body } => Thunk::Done(Ok(Rc::new(Val::UserFn {
            argname: argname.clone(),
            body: body.clone(),
            ns: Rc::new(ns.child()),
        }))),
        Expr::Call { func, arg } => {
            let func = func.clone();
            let arg = arg.clone();
            let ns2 = ns.clone();
            Thunk::NeedElaboration(
                func,
                ns,
                Box::new(move |funval| {
                    Thunk::NeedElaboration(
                        arg,
                        ns2,
                        Box::new(move |argval| hmm_call(funval, argval)),
                    )
                }),
            )
        }
    }
}

impl HmmState {
    pub fn new(expr: Rc<Expr>, ns: Rc<Ns>) -> HmmState {
        let root_marker = Marker(0);
        let mut blackboard = std::collections::HashMap::new();
        blackboard.insert(
            root_marker,
            Entry {
                need: expr,
                ns,
                callback: Box::new(|v| Thunk::Done(Ok(v))),
                parent: None,
            },
        );

        HmmState {
            last_marker: root_marker,
            steps: vec![Step::Elaborate(root_marker)],
            blackboard,
        }
    }

    pub fn step(&mut self) -> Option<EvalResult> {
        let step = self.steps.pop().expect("I'm already done!");

        match step {
            Step::Deliver(None, val) => Some(Ok(val)),
            Step::Deliver(Some(marker), val) => {
                let Entry {
                    need: _,
                    ns: _,
                    callback,
                    parent,
                } = self.blackboard.remove(&marker).unwrap();

                match callback(val) {
                    Thunk::Done(Err(e)) => {
                        self.steps = Vec::new();
                        Some(Err(e))
                    }

                    Thunk::Done(Ok(val)) => {
                        self.steps.push(Step::Deliver(parent, val));
                        None
                    }

                    Thunk::NeedElaboration(need, ns, callback) => {
                        self.last_marker.0 += 1;
                        let new_marker = self.last_marker;
                        self.blackboard.insert(
                            new_marker,
                            Entry {
                                need,
                                ns,
                                callback,
                                parent,
                            },
                        );
                        self.steps.push(Step::Elaborate(new_marker));
                        None
                    }
                }
            }
            Step::Elaborate(marker) => {
                let Entry {
                    need,
                    ns,
                    callback: _,
                    parent: _,
                } = self.blackboard.get(&marker).unwrap();

                match hmm_step(need.clone(), ns.clone()) {
                    Thunk::Done(Err(e)) => {
                        self.steps = Vec::new();
                        Some(Err(e))
                    }

                    Thunk::Done(Ok(val)) => {
                        self.steps.push(Step::Deliver(Some(marker), val));
                        None
                    }

                    Thunk::NeedElaboration(need, ns, callback) => {
                        self.last_marker.0 += 1;
                        let new_marker = self.last_marker;
                        self.blackboard.insert(
                            new_marker,
                            Entry {
                                need,
                                ns,
                                callback,
                                parent: Some(marker),
                            },
                        );
                        self.steps.push(Step::Elaborate(new_marker));
                        None
                    }
                }
            }
        }
    }
}
