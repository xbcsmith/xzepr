#!/bin/bash

# XZEPR Makefile Bash Completions
# This script generates bash completions for Makefile targets

set -euo pipefail

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to extract Makefile targets
extract_targets() {
    local makefile="${1:-Makefile}"

    if [[ ! -f "$makefile" ]]; then
        echo "Error: Makefile not found at $makefile" >&2
        return 1
    fi

    # Extract .PHONY targets and targets with ## comments
    grep -E '^[a-zA-Z_-]+:.*##|^\.PHONY:' "$makefile" | \
    grep -v '^\.PHONY:' | \
    sed 's/:.*##.*//' | \
    sed 's/:.*//' | \
    sort -u
}

# Function to generate bash completion script
generate_completions() {
    local targets
    targets=$(extract_targets)

    cat << 'EOF'
#!/bin/bash
# XZEPR Makefile Bash Completions

_xzepr_make_completions() {
    local cur prev opts
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"

    # Available targets
    opts="
EOF

    # Add targets to completion script
    echo "$targets" | while read -r target; do
        echo "        $target"
    done

    cat << 'EOF'
    "

    # Generate completions
    COMPREPLY=( $(compgen -W "${opts}" -- ${cur}) )
    return 0
}

# Register completions for make command
complete -F _xzepr_make_completions make

# Also register for common make aliases
complete -F _xzepr_make_completions m
complete -F _xzepr_make_completions gmake

# Export function for sourcing
export -f _xzepr_make_completions
EOF
}

# Function to install completions
install_completions() {
    local completion_file="$HOME/.bash_completion.d/xzepr-make"
    local bashrc_file="$HOME/.bashrc"

    # Create completion directory if it doesn't exist
    mkdir -p "$(dirname "$completion_file")"

    # Generate and save completion script
    echo -e "${BLUE}Generating bash completions...${NC}"
    generate_completions > "$completion_file"
    chmod +x "$completion_file"

    # Add source line to .bashrc if not already present
    local source_line="source $completion_file"
    if ! grep -Fq "$source_line" "$bashrc_file" 2>/dev/null; then
        echo "" >> "$bashrc_file"
        echo "# XZEPR Make completions" >> "$bashrc_file"
        echo "$source_line" >> "$bashrc_file"
        echo -e "${GREEN}Added completion source to $bashrc_file${NC}"
    fi

    echo -e "${GREEN}Bash completions installed to $completion_file${NC}"
    echo "Run 'source ~/.bashrc' or start a new shell to enable completions"
}

# Function to uninstall completions
uninstall_completions() {
    local completion_file="$HOME/.bash_completion.d/xzepr-make"
    local bashrc_file="$HOME/.bashrc"

    # Remove completion file
    if [[ -f "$completion_file" ]]; then
        rm "$completion_file"
        echo -e "${GREEN}Removed $completion_file${NC}"
    fi

    # Remove source line from .bashrc
    if [[ -f "$bashrc_file" ]]; then
        grep -v "source $completion_file" "$bashrc_file" > "${bashrc_file}.tmp" && mv "${bashrc_file}.tmp" "$bashrc_file"
        sed -i '/# XZEPR Make completions/d' "$bashrc_file"
        echo -e "${GREEN}Removed completion source from $bashrc_file${NC}"
    fi

    echo "Completions uninstalled. Restart your shell for changes to take effect."
}

# Function to test completions
test_completions() {
    echo -e "${BLUE}Testing make target completions...${NC}"
    echo "Available targets:"
    extract_targets | column -c 80
    echo ""
    echo "Try typing 'make <TAB><TAB>' to see completions"
}

# Function to show help
show_help() {
    cat << EOF
XZEPR Makefile Bash Completions

USAGE:
    $0 [COMMAND]

COMMANDS:
    install     Install bash completions for make targets
    uninstall   Remove bash completions
    test        Test and show available targets
    generate    Generate completion script to stdout
    help        Show this help message

EXAMPLES:
    # Install completions
    $0 install

    # Test completions
    $0 test

    # Generate completion script
    $0 generate > my-completions.sh

EOF
}

# Main function
main() {
    case "${1:-help}" in
        install)
            install_completions
            ;;
        uninstall)
            uninstall_completions
            ;;
        test)
            test_completions
            ;;
        generate)
            generate_completions
            ;;
        help|--help|-h)
            show_help
            ;;
        *)
            echo "Unknown command: $1" >&2
            show_help >&2
            exit 1
            ;;
    esac
}

# Run main function with all arguments
main "$@"
