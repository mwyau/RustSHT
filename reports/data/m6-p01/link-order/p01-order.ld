SECTIONS
{
  .text :
  {
    *(.text.hot)
    *(.text.*StagedSynthesisScalarTransition*)
    *(.text.*try_synthesize_order_scalar_transition_dispatched*)
  }
}
