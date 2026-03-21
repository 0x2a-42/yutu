use rustc_hash::FxHashMap;

use crate::ast::{AstNode, Block, Exp, Field, Stat};
use crate::parser::{Cst, NodeRef, Span};
use crate::sema::SemanticData;

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct BasicBlockRef(usize);

impl BasicBlockRef {
    pub const ENTRY: BasicBlockRef = BasicBlockRef(0);
}

#[derive(Debug)]
pub enum Successor {
    None,
    Uncond(BasicBlockRef),
    Cond {
        exp: Option<Exp>,
        false_bb: BasicBlockRef,
        true_bb: BasicBlockRef,
    },
}

#[derive(Debug)]
pub struct BasicBlock {
    pub span: Span,
    pub successor: Successor,
    pub reachable: bool,
}

impl BasicBlock {
    fn new() -> Self {
        Self {
            span: 0..0,
            successor: Successor::None,
            reachable: false,
        }
    }
    fn append(&mut self, span: Span) {
        if self.span.start == 0 && self.span.end == 0 {
            self.span = span;
        } else if self.span.end < span.end {
            self.span.end = span.end;
        }
    }
}

#[derive(Debug)]
pub struct ControlFlowGraph {
    pub span: Option<Span>,
    pub bbs: Vec<BasicBlock>,
    pub edges: usize,
    pub exits: usize,
}

impl ControlFlowGraph {
    fn new(span: Option<Span>) -> Self {
        Self {
            span,
            bbs: vec![BasicBlock::new()],
            edges: 0,
            exits: 0,
        }
    }
    pub fn bb_mut(&mut self, bb_ref: BasicBlockRef) -> &mut BasicBlock {
        &mut self.bbs[bb_ref.0]
    }
    fn insert_bb(&mut self) -> BasicBlockRef {
        let bb_ref = BasicBlockRef(self.bbs.len());
        self.bbs.push(BasicBlock::new());
        bb_ref
    }
    fn insert_edge(&mut self, from: BasicBlockRef, to: Successor) {
        self.edges += match to {
            Successor::Cond { .. } => 2,
            _ => 1,
        };
        self.bbs[from.0].successor = to;
    }
}

#[derive(Clone)]
enum Jumps {
    None,
    Some {
        false_bb: BasicBlockRef,
        true_bb: BasicBlockRef,
    },
}

pub struct CfgBuilder {
    cfg: ControlFlowGraph,
    cur_bb: BasicBlockRef,
    label_bbs: FxHashMap<NodeRef, BasicBlockRef>,
    break_bbs: Vec<BasicBlockRef>,
    inner_cfgs: Vec<ControlFlowGraph>,
}

impl CfgBuilder {
    pub fn build(
        cst: &Cst<'_>,
        sema: &SemanticData,
        block: Block,
        span: Option<Span>,
    ) -> Vec<ControlFlowGraph> {
        let mut builder = Self {
            cfg: ControlFlowGraph::new(span),
            cur_bb: BasicBlockRef(0),
            label_bbs: FxHashMap::default(),
            break_bbs: Vec::new(),
            inner_cfgs: Vec::new(),
        };
        builder.visit_block(cst, sema, block);
        // TODO: reorder blocks to be in reverse postorder, so forward analysis is faster
        builder.inner_cfgs.push(builder.cfg);
        builder.inner_cfgs
    }

    fn visit_func(&mut self, cst: &Cst<'_>, sema: &SemanticData, block: Block, span: Option<Span>) {
        let cfgs = CfgBuilder::build(cst, sema, block, span);
        self.inner_cfgs.extend(cfgs);
    }
    fn visit_block(&mut self, cst: &Cst<'_>, sema: &SemanticData, block: Block) {
        for stat in block.stats(cst) {
            self.visit_statement(cst, sema, stat);
        }
    }
    fn visit_statement(&mut self, cst: &Cst<'_>, sema: &SemanticData, stat: Stat) {
        match stat {
            Stat::Emptystat(emptystat) => {
                self.cfg.bb_mut(self.cur_bb).append(emptystat.span(cst));
            }
            Stat::Expstat(expstat) => {
                self.cfg.bb_mut(self.cur_bb).append(expstat.span(cst));
                if let Some(exp) = expstat.exp(cst) {
                    self.visit_exp(cst, sema, exp, Jumps::None);
                }
                if let Some(Exp::Callexp(callexp)) = expstat.exp(cst)
                    && let Some(Exp::Nameexp(nameexp)) = callexp.base(cst)
                    && let Some((name, _)) = nameexp.name(cst)
                    && let Some(args) = callexp.args(cst)
                {
                    // TODO: check for redefined std methods like assert, error
                    let terminator = match name {
                        "assert" => matches!(
                            args.expressions(cst).next(),
                            Some(Exp::Falseexp(_) | Exp::Nilexp(_))
                        ),
                        "error" => true,
                        _ => false,
                    };
                    if terminator {
                        self.cur_bb = self.cfg.insert_bb();
                        self.cfg.exits += 1;
                    }
                }
            }
            Stat::Assignstat(assignstat) => {
                self.cfg.bb_mut(self.cur_bb).append(assignstat.span(cst));
                for exp in assignstat.rhs_exps(cst) {
                    self.visit_exp(cst, sema, exp, Jumps::None);
                }
                for exp in assignstat.lhs_exps(cst) {
                    self.visit_exp(cst, sema, exp, Jumps::None);
                }
            }
            Stat::Label(label) => {
                let span = label.span(cst);
                let label_bb = *self
                    .label_bbs
                    .entry(label.syntax())
                    .or_insert_with(|| self.cfg.insert_bb());
                self.cfg
                    .insert_edge(self.cur_bb, Successor::Uncond(label_bb));
                self.cur_bb = label_bb;
                self.cfg.bb_mut(self.cur_bb).append(span.clone());
            }
            Stat::Breakstat(breakstat) => {
                let span = breakstat.span(cst);
                self.cfg.bb_mut(self.cur_bb).append(span.clone());
                if let Some(break_bb) = self.break_bbs.last() {
                    self.cfg
                        .insert_edge(self.cur_bb, Successor::Uncond(*break_bb));
                }
                self.cur_bb = self.cfg.insert_bb();
            }
            Stat::Gotostat(gotostat) => {
                let span = gotostat.span(cst);
                self.cfg.bb_mut(self.cur_bb).append(span.clone());
                if let Some(label) = sema.decl_bindings.get(&gotostat.syntax()) {
                    let label_bb = *self
                        .label_bbs
                        .entry(*label)
                        .or_insert_with(|| self.cfg.insert_bb());
                    self.cfg
                        .insert_edge(self.cur_bb, Successor::Uncond(label_bb));
                }
                self.cur_bb = self.cfg.insert_bb();
            }
            Stat::Dostat(dostat) => {
                self.cfg.bb_mut(self.cur_bb).append(dostat.span(cst));
            }
            Stat::Whilestat(whilestat) => {
                self.cfg.bb_mut(self.cur_bb).append(whilestat.span(cst));

                let merge_bb = self.cfg.insert_bb();
                let block_bb = self.cfg.insert_bb();
                let cond_bb = self.cfg.insert_bb();

                self.cfg
                    .insert_edge(self.cur_bb, Successor::Uncond(cond_bb));
                if let Some(cond) = whilestat.cond(cst) {
                    self.cur_bb = cond_bb;
                    self.visit_exp(
                        cst,
                        sema,
                        cond,
                        Jumps::Some {
                            false_bb: merge_bb,
                            true_bb: block_bb,
                        },
                    );
                }
                if let Some(block) = whilestat.block(cst) {
                    self.cur_bb = block_bb;
                    self.break_bbs.push(merge_bb);
                    self.visit_block(cst, sema, block);
                    self.break_bbs.pop();
                    self.cfg
                        .insert_edge(self.cur_bb, Successor::Uncond(cond_bb));
                }
                self.cur_bb = merge_bb;
            }
            Stat::Repeatstat(repeatstat) => {
                self.cfg.bb_mut(self.cur_bb).append(repeatstat.span(cst));

                let merge_bb = self.cfg.insert_bb();
                let block_bb = self.cfg.insert_bb();
                let cond_bb = self.cfg.insert_bb();

                self.cfg
                    .insert_edge(self.cur_bb, Successor::Uncond(block_bb));
                if let Some(block) = repeatstat.block(cst) {
                    self.cur_bb = block_bb;
                    self.break_bbs.push(merge_bb);
                    self.visit_block(cst, sema, block);
                    self.break_bbs.pop();
                    self.cfg
                        .insert_edge(self.cur_bb, Successor::Uncond(cond_bb));
                }
                if let Some(cond) = repeatstat.cond(cst) {
                    self.cur_bb = cond_bb;
                    self.visit_exp(
                        cst,
                        sema,
                        cond,
                        Jumps::Some {
                            false_bb: block_bb,
                            true_bb: merge_bb,
                        },
                    );
                }
                self.cur_bb = merge_bb;
            }
            Stat::Ifstat(ifstat) => {
                self.cfg.bb_mut(self.cur_bb).append(ifstat.span(cst));

                let merge_bb = self.cfg.insert_bb();
                let visit_branch = |builder: &mut Self, cond, block, else_bb| {
                    let entry_bb = builder.cfg.insert_bb();
                    if let Some(cond) = cond {
                        builder.visit_exp(
                            cst,
                            sema,
                            cond,
                            Jumps::Some {
                                false_bb: else_bb,
                                true_bb: entry_bb,
                            },
                        );
                    }
                    if let Some(block) = block {
                        builder.cur_bb = entry_bb;
                        builder.visit_block(cst, sema, block);
                        builder
                            .cfg
                            .insert_edge(builder.cur_bb, Successor::Uncond(merge_bb));
                    }
                };

                let elif_branches = ifstat.elif_branches(cst).collect::<Vec<_>>();
                let else_branch = ifstat.else_branch(cst);

                let mut else_bb;
                if !elif_branches.is_empty() || else_branch.is_some() {
                    else_bb = self.cfg.insert_bb();
                } else {
                    else_bb = merge_bb;
                }
                visit_branch(self, ifstat.cond(cst), ifstat.then_block(cst), else_bb);

                for (i, elif_branch) in elif_branches.iter().enumerate() {
                    self.cur_bb = else_bb;
                    if i < elif_branches.len() - 1 || else_branch.is_some() {
                        else_bb = self.cfg.insert_bb();
                    } else {
                        else_bb = merge_bb;
                    }
                    visit_branch(self, elif_branch.cond(cst), elif_branch.block(cst), else_bb);
                }
                if let Some(else_branch) = else_branch
                    && let Some(block) = else_branch.block(cst)
                {
                    self.cur_bb = else_bb;
                    self.visit_block(cst, sema, block);
                    self.cfg
                        .insert_edge(self.cur_bb, Successor::Uncond(merge_bb));
                }
                self.cur_bb = merge_bb;
            }
            Stat::NumericForstat(forstat) => {
                self.cfg.bb_mut(self.cur_bb).append(forstat.span(cst));

                let merge_bb = self.cfg.insert_bb();
                let block_bb = self.cfg.insert_bb();

                let (from, to, stride) = forstat.exp_from_to_stride(cst);
                if let Some(from) = from {
                    self.visit_exp(cst, sema, from, Jumps::None);
                }
                if let Some(to) = to {
                    self.visit_exp(cst, sema, to, Jumps::None);
                }
                if let Some(stride) = stride {
                    self.visit_exp(cst, sema, stride, Jumps::None);
                }

                self.cfg
                    .insert_edge(self.cur_bb, Successor::Uncond(block_bb));
                if let Some(block) = forstat.block(cst) {
                    self.cur_bb = block_bb;
                    self.break_bbs.push(merge_bb);
                    self.visit_block(cst, sema, block);
                    self.break_bbs.pop();
                    self.cfg.insert_edge(
                        self.cur_bb,
                        Successor::Cond {
                            exp: None,
                            true_bb: block_bb,
                            false_bb: merge_bb,
                        },
                    );
                }
                self.cur_bb = merge_bb;
            }
            Stat::GenericForstat(forstat) => {
                self.cfg.bb_mut(self.cur_bb).append(forstat.span(cst));

                let merge_bb = self.cfg.insert_bb();
                let block_bb = self.cfg.insert_bb();

                for exp in forstat.exps(cst) {
                    self.visit_exp(cst, sema, exp, Jumps::None);
                }

                self.cfg.insert_edge(
                    self.cur_bb,
                    Successor::Cond {
                        exp: None,
                        true_bb: block_bb,
                        false_bb: merge_bb,
                    },
                );
                if let Some(block) = forstat.block(cst) {
                    self.cur_bb = block_bb;
                    self.break_bbs.push(merge_bb);
                    self.visit_block(cst, sema, block);
                    self.break_bbs.pop();
                    self.cfg.insert_edge(
                        self.cur_bb,
                        Successor::Cond {
                            exp: None,
                            true_bb: block_bb,
                            false_bb: merge_bb,
                        },
                    );
                }
                self.cur_bb = merge_bb;
            }
            Stat::Funcstat(funcstat) => {
                self.cfg.bb_mut(self.cur_bb).append(funcstat.span(cst));
                if let Some(block) = funcstat.block(cst) {
                    self.visit_func(cst, sema, block, Some(funcstat.span(cst)));
                }
            }
            Stat::Localvarstat(localvarstat) => {
                self.cfg.bb_mut(self.cur_bb).append(localvarstat.span(cst));
            }
            Stat::Localfuncstat(localfuncstat) => {
                self.cfg.bb_mut(self.cur_bb).append(localfuncstat.span(cst));
                if let Some(block) = localfuncstat.block(cst) {
                    self.visit_func(cst, sema, block, Some(localfuncstat.span(cst)));
                }
            }
            Stat::Globalvarstat(globalvarstat) => {
                self.cfg.bb_mut(self.cur_bb).append(globalvarstat.span(cst));
            }
            Stat::Globalfuncstat(globalfuncstat) => {
                self.cfg
                    .bb_mut(self.cur_bb)
                    .append(globalfuncstat.span(cst));
                if let Some(block) = globalfuncstat.block(cst) {
                    self.visit_func(cst, sema, block, Some(globalfuncstat.span(cst)));
                }
            }
            Stat::CollectiveGlobalvarstat(collective_globalvarstat) => {
                self.cfg
                    .bb_mut(self.cur_bb)
                    .append(collective_globalvarstat.span(cst));
            }
            Stat::Retstat(retstat) => {
                let span = retstat.span(cst);
                self.cfg.bb_mut(self.cur_bb).append(span);
                self.cur_bb = self.cfg.insert_bb();
                self.cfg.exits += 1;
            }
        }
    }
    fn visit_exp(&mut self, cst: &Cst<'_>, sema: &SemanticData, exp: Exp, jumps: Jumps) {
        let truthy = |cur_bb, cfg: &mut ControlFlowGraph| {
            if let Jumps::Some { true_bb, .. } = jumps {
                cfg.insert_edge(cur_bb, Successor::Uncond(true_bb));
            }
        };
        let undecided = |cur_bb, cfg: &mut ControlFlowGraph| {
            if let Jumps::Some { false_bb, true_bb } = jumps {
                cfg.insert_edge(
                    cur_bb,
                    Successor::Cond {
                        exp: Some(exp),
                        false_bb,
                        true_bb,
                    },
                );
            }
        };
        match exp {
            Exp::Binexp(binexp) => {
                let mut ops = binexp.operands(cst);
                let lhs = ops.next();
                let rhs = ops.next();

                if let Some(lhs) = lhs
                    && let Some(rhs) = rhs
                {
                    let is_and = binexp.is_and(cst);
                    let is_or = binexp.is_or(cst);
                    let outside_ctrl_flow = matches!(jumps, Jumps::None);
                    let jumps = if outside_ctrl_flow && (is_and || is_or) {
                        let complete_bb = self.cfg.insert_bb();
                        Jumps::Some {
                            false_bb: complete_bb,
                            true_bb: complete_bb,
                        }
                    } else {
                        jumps.clone()
                    };

                    if let Jumps::Some { false_bb, true_bb } = jumps {
                        if is_and {
                            let rhs_bb = self.cfg.insert_bb();
                            self.cfg.bb_mut(self.cur_bb).append(lhs.span(cst));
                            self.visit_exp(
                                cst,
                                sema,
                                lhs,
                                Jumps::Some {
                                    false_bb,
                                    true_bb: rhs_bb,
                                },
                            );
                            self.cur_bb = rhs_bb;
                            self.cfg.bb_mut(self.cur_bb).append(rhs.span(cst));
                            self.visit_exp(cst, sema, rhs, Jumps::Some { false_bb, true_bb });
                        } else if is_or {
                            let rhs_bb = self.cfg.insert_bb();
                            self.cfg.bb_mut(self.cur_bb).append(lhs.span(cst));
                            self.visit_exp(
                                cst,
                                sema,
                                lhs,
                                Jumps::Some {
                                    false_bb: rhs_bb,
                                    true_bb,
                                },
                            );
                            self.cur_bb = rhs_bb;
                            self.cfg.bb_mut(self.cur_bb).append(rhs.span(cst));
                            self.visit_exp(cst, sema, rhs, Jumps::Some { false_bb, true_bb });
                        } else {
                            self.visit_exp(cst, sema, lhs, Jumps::None);
                            self.visit_exp(cst, sema, rhs, Jumps::None);
                            undecided(self.cur_bb, &mut self.cfg);
                        }
                    } else {
                        self.visit_exp(cst, sema, lhs, Jumps::None);
                        self.visit_exp(cst, sema, rhs, Jumps::None);
                    }

                    if outside_ctrl_flow && (is_and || is_or) {
                        let Jumps::Some { true_bb, .. } = jumps else {
                            unreachable!()
                        };
                        self.cur_bb = true_bb;
                    }
                }
            }
            Exp::Unaryexp(unaryexp) => {
                if let Some(operand) = unaryexp.operand(cst) {
                    if unaryexp.is_negation(cst) {
                        if let Jumps::Some { false_bb, true_bb } = jumps {
                            self.visit_exp(
                                cst,
                                sema,
                                operand,
                                Jumps::Some {
                                    false_bb: true_bb,
                                    true_bb: false_bb,
                                },
                            );
                        }
                    } else {
                        self.visit_exp(cst, sema, operand, jumps);
                    }
                }
            }
            Exp::Nameexp(_) | Exp::Varargexp(_) => undecided(self.cur_bb, &mut self.cfg),
            Exp::Fieldexp(fieldexp) => {
                if let Some(base) = fieldexp.base(cst) {
                    self.visit_exp(cst, sema, base, Jumps::None);
                }
                undecided(self.cur_bb, &mut self.cfg);
            }
            Exp::Indexexp(indexexp) => {
                let (base, index) = indexexp.base_and_index(cst);
                if let Some(base) = base {
                    self.visit_exp(cst, sema, base, Jumps::None);
                }
                if let Some(index) = index {
                    self.visit_exp(cst, sema, index, Jumps::None);
                }
                undecided(self.cur_bb, &mut self.cfg);
            }
            Exp::Callexp(callexp) => {
                if let Some(base) = callexp.base(cst) {
                    self.visit_exp(cst, sema, base, Jumps::None);
                }
                if let Some(args) = callexp.args(cst) {
                    for argexp in args.expressions(cst) {
                        self.visit_exp(cst, sema, argexp, Jumps::None);
                    }
                }
                undecided(self.cur_bb, &mut self.cfg);
            }
            Exp::Tableconstructor(tableconstructor) => {
                for field in tableconstructor.fields(cst) {
                    match field {
                        Field::KeyvalField(keyval_field) => {
                            let (key, val) = keyval_field.key_val(cst);
                            if let Some(key) = key {
                                self.visit_exp(cst, sema, key, Jumps::None);
                            }
                            if let Some(val) = val {
                                self.visit_exp(cst, sema, val, Jumps::None);
                            }
                        }
                        Field::NamedField(named_field) => {
                            let val = named_field.val(cst);
                            if let Some(val) = val {
                                self.visit_exp(cst, sema, val, Jumps::None);
                            }
                        }
                        Field::CountedField(counted_field) => {
                            let val = counted_field.val(cst);
                            if let Some(val) = val {
                                self.visit_exp(cst, sema, val, Jumps::None);
                            }
                        }
                    }
                }
                truthy(self.cur_bb, &mut self.cfg);
            }
            Exp::Functiondef(functiondef) => {
                if let Some(block) = functiondef.block(cst) {
                    self.visit_func(cst, sema, block, Some(functiondef.span(cst)));
                }
                truthy(self.cur_bb, &mut self.cfg);
            }
            Exp::Stringexp(_)
            | Exp::Decintexp(_)
            | Exp::Hexintexp(_)
            | Exp::Decfloatexp(_)
            | Exp::Hexfloatexp(_)
            | Exp::Trueexp(_) => truthy(self.cur_bb, &mut self.cfg),
            Exp::Falseexp(_) | Exp::Nilexp(_) => {
                if let Jumps::Some { false_bb, .. } = jumps {
                    self.cfg
                        .insert_edge(self.cur_bb, Successor::Uncond(false_bb));
                }
            }
            Exp::Parenexp(parenexp) => {
                if let Some(inner) = parenexp.inner(cst) {
                    self.visit_exp(cst, sema, inner, jumps);
                }
            }
        }
    }
}
