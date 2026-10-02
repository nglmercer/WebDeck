fn invoke_action(action, args) {
    if action != "echo" { throw "Unknown action"; }
    invoke(#{type: "debug", data: #{text: args.text}})
}
