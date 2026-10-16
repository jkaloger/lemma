# Sourced by demo.tape: fake lemma alias and a starship-style prompt.

alias lemma="$PWD/scripts/demo.sh"

__demo_rust=$(rustc --version 2>/dev/null | awk '{ print $2 }')
__demo_branch=$(git branch --show-current 2>/dev/null)

PS1='\[\e[1;36m\]lemma\[\e[0m\] on \[\e[1;35m\]'"$__demo_branch"'\[\e[0m\] via \[\e[1;31m\]🦀 v'"$__demo_rust"'\[\e[0m\]\n\[\e[1;32m\]❯\[\e[0m\] '
clear
