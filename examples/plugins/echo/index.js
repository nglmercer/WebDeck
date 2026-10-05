export function invoke_action(action, args, ctx) {
  return ctx.invoke({type: 'debug', data: {text: args.text}});
}
