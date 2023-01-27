//! Direct evaluation strategy. A dead simple recursive tree walker.
//!
//! Just call `direct_eval` with an expression and the built-in namespace,
//! and before you know it you get the result.
//!

use crate::base::EvalError;
use std::rc::Rc;

pub type Val = crate::base::Val<Builtin>;
pub type Expr = crate::base::Expr<Builtin>;
pub type Ns = crate::base::Ns<Builtin>;
pub type EvalResult = crate::base::EvalResult<Builtin>;

pub struct Builtin(Rc<Box<dyn Fn(Rc<Val>) -> EvalResult>>);

pub fn direct_eval(expr: Rc<Expr>, ns: Rc<Ns>) -> EvalResult {
    match expr.as_ref() {
        Expr::Put(val) => Ok(val.clone()),
        Expr::Name(name) => match ns.lookup(name.clone()) {
            Some(v) => Ok(v),
            None => Err(EvalError::UndefinedName(name.as_ref().clone())),
        },
        Expr::Call { func, arg } => {
            let func = direct_eval(func.clone(), ns.clone())?;
            let arg = direct_eval(arg.clone(), ns)?;

            match func.as_ref() {
                Val::Builtin(_, Builtin(f)) => f(arg),
                Val::UserFn {
                    argname,
                    body,
                    ns: closure,
                } => {
                    let local_scope = closure.clone().child().update(argname.clone(), arg);
                    direct_eval(body.clone(), Rc::new(local_scope))
                }
                Val::Int(_) => Err(EvalError::TypeMismatch("Cannot call integer")),
                Val::Str(_) => Err(EvalError::TypeMismatch("Cannot call string")),
                Val::Vec(_) => Err(EvalError::TypeMismatch("Cannot call vec")),
            }
        }
        Expr::Lam { argname, body } => Ok(Rc::new(Val::UserFn {
            argname: argname.clone(),
            body: body.clone(),
            ns: Rc::new(ns.child()),
        })),
    }
}
