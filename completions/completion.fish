set -l commands sync import clone

complete -c src-manage -f

complete -c src-manage -n "not __fish_seen_subcommand_from $commands" \
    -a "sync import clone"

complete -c src-manage -n "__fish_seen_subcommand_from sync" \
    -a "(__fish_print_hostnames)"

complete -c src-manage -n "__fish_seen_subcommand_from import" \
    -a "(__fish_complete_directories)"
