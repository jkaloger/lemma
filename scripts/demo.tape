# vhs scripts/demo.tape  (run from the repo root)

Output docs/assets/demo.gif

Set Shell "bash"
Set FontSize 48
Set Width 2640
Set Height 1640
Set Padding 48
Set Margin 96
Set MarginFill "#b3a1e6"
Set BorderRadius 24
Set WindowBar Colorful
Set WindowBarSize 80
# Spaceduck. For a built-in theme: Set Theme "GitHub Dark" (list them with `vhs themes`)
Set Theme { "name": "Spaceduck", "background": "#0f111b", "foreground": "#ecf0c1", "cursor": "#ecf0c1", "selection": "#686f9a", "black": "#000000", "red": "#e33400", "green": "#5ccc96", "yellow": "#b3a1e6", "blue": "#00a3cc", "magenta": "#ce6f8f", "cyan": "#7a5ccc", "white": "#686f9a", "brightBlack": "#686f9a", "brightRed": "#e33400", "brightGreen": "#5ccc96", "brightYellow": "#b3a1e6", "brightBlue": "#00a3cc", "brightMagenta": "#ce6f8f", "brightCyan": "#7a5ccc", "brightWhite": "#f0f1ce" }
Set TypingSpeed 60ms

Hide
Type "source scripts/demo-env.sh"
Enter
Show

Type "lemma lint"
Sleep 400ms
Enter
Wait+Screen@10s /confirm intended/
Sleep 2s

Type "lemma plan DELTA-123"
Sleep 400ms
Enter
Wait+Screen@30s /next:/
Sleep 2s

Type "lemma build DELTA-123"
Sleep 400ms
Enter

Wait+Screen@60s /all tasks complete/
Sleep 3s
Screenshot docs/assets/demo.png
