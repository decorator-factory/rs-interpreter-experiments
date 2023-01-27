use std::rc::Rc;

use interpreter_experiments::hmm_eval::{Expr, HmmState, Ns, Val};
use interpreter_experiments::F;

fn main() {
    let nil_closure = Rc::new(Ns::new());

    let builtins = Ns::new()
        .entry(
            "tru",
            Val::UserFn {
                argname: Rc::new("x".into()),
                body: Rc::new(Expr::Lam {
                    argname: Rc::new("__y".into()),
                    body: Rc::new(Expr::Name(Rc::new("x".into()))),
                }),
                ns: nil_closure.clone(),
            },
        )
        .entry(
            "lie",
            Val::UserFn {
                argname: Rc::new("__x".into()),
                body: Rc::new(Expr::Lam {
                    argname: Rc::new("y".into()),
                    body: Rc::new(Expr::Name(Rc::new("y".into()))),
                }),
                ns: nil_closure,
            },
        );

    let expr = F!(
        'app F!('app F![lie], 'to F![420]),
        'to F!(
            'app F!('app F![tru], 'to F![55]),
            'to F!('lam x . F!('app F![x], 'to F![lie]))));

    let mut state = HmmState::new(expr, Rc::new(builtins));

    let mut step = 0;
    let result = loop {
        step += 1;
        if let Some(r) = state.step() {
            break r;
        }
    };

    println!("Result: {:?}", result);
    println!("Steps: {:?}", &step);
}
