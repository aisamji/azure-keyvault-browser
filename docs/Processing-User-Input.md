# Processing User Input

User input handled by the `App.process_terminal_event` function in the `tui` module. The `input::forwarder` simply watches for `crossterm::Events` in a loop and forwards them to the tui thread for processing. There should be no reason to make any changes to the input module other than bugfixes.

User input is somewhat of a broad definition here as resizing the terminal or alt-tabbing away from the terminal is also classified as user input. The `crossterm` terminal library converts each of these into `crossterm::Event` variant. This `Event` can than be handled by `App.process_terminal_event` or by another function that is called by the `process_terminal_event`.

The `process_terminal_event` function may make use of the app's current state in conjunction with the received `Event` to either modify the state further, indicate that the app should quit, or launch a background task as discussed in [Running-Background-Tasks](Running-Background-Tasks.md)

