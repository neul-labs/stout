# Print an optspec for argparse to handle cmd's options that are independent of any subcommand.
function __fish_stout_global_optspecs
	string join \n v/verbose q/quiet h/help V/version
end

function __fish_stout_needs_command
	# Figure out if the current invocation already has a command.
	set -l cmd (commandline -opc)
	set -e cmd[1]
	argparse -s (__fish_stout_global_optspecs) -- $cmd 2>/dev/null
	or return
	if set -q argv[1]
		# Also print the command, so this can be used to figure out what it is.
		echo $argv[1]
		return 1
	end
	return 0
end

function __fish_stout_using_subcommand
	set -l cmd (__fish_stout_needs_command)
	test -z "$cmd"
	and return 1
	contains -- $cmd[1] $argv
end

complete -c stout -n "__fish_stout_needs_command" -s v -l verbose -d 'Enable verbose output'
complete -c stout -n "__fish_stout_needs_command" -s q -l quiet -d 'Suppress output'
complete -c stout -n "__fish_stout_needs_command" -s h -l help -d 'Print help'
complete -c stout -n "__fish_stout_needs_command" -s V -l version -d 'Print version'
complete -c stout -n "__fish_stout_needs_command" -f -a "install" -d 'Install packages'
complete -c stout -n "__fish_stout_needs_command" -f -a "uninstall" -d 'Uninstall packages'
complete -c stout -n "__fish_stout_needs_command" -f -a "search" -d 'Search for packages'
complete -c stout -n "__fish_stout_needs_command" -f -a "info" -d 'Show package information'
complete -c stout -n "__fish_stout_needs_command" -f -a "list" -d 'List installed packages'
complete -c stout -n "__fish_stout_needs_command" -f -a "update" -d 'Update the formula index'
complete -c stout -n "__fish_stout_needs_command" -f -a "upgrade" -d 'Upgrade installed packages'
complete -c stout -n "__fish_stout_needs_command" -f -a "doctor" -d 'Check system health'
complete -c stout -n "__fish_stout_needs_command" -f -a "completions" -d 'Generate shell completions'
complete -c stout -n "__fish_stout_needs_command" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c stout -n "__fish_stout_using_subcommand install" -l ignore-dependencies -d 'Don\'t install dependencies'
complete -c stout -n "__fish_stout_using_subcommand install" -l dry-run -d 'Show what would be done without doing it'
complete -c stout -n "__fish_stout_using_subcommand install" -s v -l verbose -d 'Enable verbose output'
complete -c stout -n "__fish_stout_using_subcommand install" -s q -l quiet -d 'Suppress output'
complete -c stout -n "__fish_stout_using_subcommand install" -s h -l help -d 'Print help'
complete -c stout -n "__fish_stout_using_subcommand uninstall" -l force -d 'Remove even if other packages depend on it'
complete -c stout -n "__fish_stout_using_subcommand uninstall" -l dry-run -d 'Show what would be done without doing it'
complete -c stout -n "__fish_stout_using_subcommand uninstall" -s v -l verbose -d 'Enable verbose output'
complete -c stout -n "__fish_stout_using_subcommand uninstall" -s q -l quiet -d 'Suppress output'
complete -c stout -n "__fish_stout_using_subcommand uninstall" -s h -l help -d 'Print help'
complete -c stout -n "__fish_stout_using_subcommand search" -s l -l limit -d 'Maximum results to show' -r
complete -c stout -n "__fish_stout_using_subcommand search" -s v -l verbose -d 'Enable verbose output'
complete -c stout -n "__fish_stout_using_subcommand search" -s q -l quiet -d 'Suppress output'
complete -c stout -n "__fish_stout_using_subcommand search" -s h -l help -d 'Print help'
complete -c stout -n "__fish_stout_using_subcommand info" -s v -l verbose -d 'Enable verbose output'
complete -c stout -n "__fish_stout_using_subcommand info" -s q -l quiet -d 'Suppress output'
complete -c stout -n "__fish_stout_using_subcommand info" -s h -l help -d 'Print help'
complete -c stout -n "__fish_stout_using_subcommand list" -s v -l versions -d 'Show versions only'
complete -c stout -n "__fish_stout_using_subcommand list" -s p -l paths -d 'Show full paths'
complete -c stout -n "__fish_stout_using_subcommand list" -s v -l verbose -d 'Enable verbose output'
complete -c stout -n "__fish_stout_using_subcommand list" -s q -l quiet -d 'Suppress output'
complete -c stout -n "__fish_stout_using_subcommand list" -s h -l help -d 'Print help'
complete -c stout -n "__fish_stout_using_subcommand update" -s f -l force -d 'Force update even if index is fresh'
complete -c stout -n "__fish_stout_using_subcommand update" -s v -l verbose -d 'Enable verbose output'
complete -c stout -n "__fish_stout_using_subcommand update" -s q -l quiet -d 'Suppress output'
complete -c stout -n "__fish_stout_using_subcommand update" -s h -l help -d 'Print help'
complete -c stout -n "__fish_stout_using_subcommand upgrade" -l dry-run -d 'Show what would be done without doing it'
complete -c stout -n "__fish_stout_using_subcommand upgrade" -s v -l verbose -d 'Enable verbose output'
complete -c stout -n "__fish_stout_using_subcommand upgrade" -s q -l quiet -d 'Suppress output'
complete -c stout -n "__fish_stout_using_subcommand upgrade" -s h -l help -d 'Print help'
complete -c stout -n "__fish_stout_using_subcommand doctor" -s v -l verbose -d 'Enable verbose output'
complete -c stout -n "__fish_stout_using_subcommand doctor" -s q -l quiet -d 'Suppress output'
complete -c stout -n "__fish_stout_using_subcommand doctor" -s h -l help -d 'Print help'
complete -c stout -n "__fish_stout_using_subcommand completions" -s v -l verbose -d 'Enable verbose output'
complete -c stout -n "__fish_stout_using_subcommand completions" -s q -l quiet -d 'Suppress output'
complete -c stout -n "__fish_stout_using_subcommand completions" -s h -l help -d 'Print help'
complete -c stout -n "__fish_stout_using_subcommand help; and not __fish_seen_subcommand_from install uninstall search info list update upgrade doctor completions help" -f -a "install" -d 'Install packages'
complete -c stout -n "__fish_stout_using_subcommand help; and not __fish_seen_subcommand_from install uninstall search info list update upgrade doctor completions help" -f -a "uninstall" -d 'Uninstall packages'
complete -c stout -n "__fish_stout_using_subcommand help; and not __fish_seen_subcommand_from install uninstall search info list update upgrade doctor completions help" -f -a "search" -d 'Search for packages'
complete -c stout -n "__fish_stout_using_subcommand help; and not __fish_seen_subcommand_from install uninstall search info list update upgrade doctor completions help" -f -a "info" -d 'Show package information'
complete -c stout -n "__fish_stout_using_subcommand help; and not __fish_seen_subcommand_from install uninstall search info list update upgrade doctor completions help" -f -a "list" -d 'List installed packages'
complete -c stout -n "__fish_stout_using_subcommand help; and not __fish_seen_subcommand_from install uninstall search info list update upgrade doctor completions help" -f -a "update" -d 'Update the formula index'
complete -c stout -n "__fish_stout_using_subcommand help; and not __fish_seen_subcommand_from install uninstall search info list update upgrade doctor completions help" -f -a "upgrade" -d 'Upgrade installed packages'
complete -c stout -n "__fish_stout_using_subcommand help; and not __fish_seen_subcommand_from install uninstall search info list update upgrade doctor completions help" -f -a "doctor" -d 'Check system health'
complete -c stout -n "__fish_stout_using_subcommand help; and not __fish_seen_subcommand_from install uninstall search info list update upgrade doctor completions help" -f -a "completions" -d 'Generate shell completions'
complete -c stout -n "__fish_stout_using_subcommand help; and not __fish_seen_subcommand_from install uninstall search info list update upgrade doctor completions help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
