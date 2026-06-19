_rpass() {
    local cur="${COMP_WORDS[COMP_CWORD]}"
    local prev="${COMP_WORDS[COMP_CWORD-1]}"

    case "$prev" in
        show|insert|edit|rm|generate|otp|mv)
            mapfile -t COMPREPLY < <(rpass complete-entries -- "$cur" 2>/dev/null)
            return
            ;;
    esac

    local subcommands="list show init recipients insert edit rm mv git generate otp search doctor completions"
    COMPREPLY=($(compgen -W "$subcommands" -- "$cur"))
}
complete -F _rpass rpass
