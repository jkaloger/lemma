# vhs scripts/demo-build.tape  (run from the repo root)

Output docs/assets/build.gif

Set Shell "bash"
Set FontSize 16
Set Width 1200
Set Height 520
Set Padding 24
# Spaceduck. For a built-in theme: Set Theme "GitHub Dark" (list them with `vhs themes`)
Set Theme { "name": "Spaceduck", "background": "#0f111b", "foreground": "#ecf0c1", "cursor": "#ecf0c1", "selection": "#686f9a", "black": "#000000", "red": "#e33400", "green": "#5ccc96", "yellow": "#b3a1e6", "blue": "#00a3cc", "magenta": "#ce6f8f", "cyan": "#7a5ccc", "white": "#686f9a", "brightBlack": "#686f9a", "brightRed": "#e33400", "brightGreen": "#5ccc96", "brightYellow": "#b3a1e6", "brightBlue": "#00a3cc", "brightMagenta": "#ce6f8f", "brightCyan": "#7a5ccc", "brightWhite": "#f0f1ce" }
Set TypingSpeed 60ms

Hide
Type "source scripts/demo-env.sh"
Enter
Show

Type "lemma build DELTA-123"
Sleep 400ms
Enter

Wait+Screen@60s /all tasks complete/
Sleep 3s
Screenshot docs/assets/build.png
