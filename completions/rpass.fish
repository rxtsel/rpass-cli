function __rpass_entry_commands
    contains -- (commandline -opc)[2] show insert edit rm mv generate otp
end

complete -c rpass -f

complete -c rpass -n '__rpass_entry_commands' -a '(rpass complete-entries -- (commandline -ct) 2>/dev/null)'

complete -c rpass -n '__fish_use_subcommand' -a 'list' -d 'List password store entries'
complete -c rpass -n '__fish_use_subcommand' -a 'show' -d 'Show a password store entry'
complete -c rpass -n '__fish_use_subcommand' -a 'init' -d 'Initialize a password store or subfolder'
complete -c rpass -n '__fish_use_subcommand' -a 'recipients' -d 'List or modify password store recipients'
complete -c rpass -n '__fish_use_subcommand' -a 'insert' -d 'Insert a password store entry'
complete -c rpass -n '__fish_use_subcommand' -a 'edit' -d 'Edit a password store entry'
complete -c rpass -n '__fish_use_subcommand' -a 'rm' -d 'Remove a password store entry'
complete -c rpass -n '__fish_use_subcommand' -a 'mv' -d 'Move or rename a password store entry'
complete -c rpass -n '__fish_use_subcommand' -a 'git' -d 'Run git inside the password store'
complete -c rpass -n '__fish_use_subcommand' -a 'generate' -d 'Generate and insert a password store entry'
complete -c rpass -n '__fish_use_subcommand' -a 'otp' -d 'Generate an OTP code for a password store entry'
complete -c rpass -n '__fish_use_subcommand' -a 'search' -d 'Search password store entries'
complete -c rpass -n '__fish_use_subcommand' -a 'doctor' -d 'Check the local rpass environment'
complete -c rpass -n '__fish_use_subcommand' -a 'completions' -d 'Generate shell completion scripts'
