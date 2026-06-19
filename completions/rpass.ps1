$entryCommands = @('show', 'insert', 'edit', 'rm', 'mv', 'generate', 'otp')

Register-ArgumentCompleter -Native -CommandName rpass -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $tokens = $commandAst.CommandElements | ForEach-Object { $_.Extent.Text }
    $subcommand = if ($tokens.Count -ge 2) { $tokens[1] } else { '' }

    if ($entryCommands -notcontains $subcommand) {
        return
    }

    try {
        & rpass complete-entries -- $wordToComplete 2>$null | ForEach-Object {
            [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
        }
    } catch {
        return
    }
}
