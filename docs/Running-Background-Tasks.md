# Running Background Tasks

In order to keep the TUI responsive, long-running or asynchronous operations should be launched in background tasks. We do this by leveraging the [tokio](https://docs.rs/tokio/latest/tokio/index.html) runtime. All new background tasks must define a `TaskSpec` and may also define a `TuiEvent`.

## Defining the Background Task

A background task is defined by a function and a `TaskSpec` in the `background` module. The `TaskSpec` variant should be designed to accept the parameters for the task. The `background::manager` should then be updated to launch the appropriate function using `tokio::task::spawn` or `tokio::task::spawn_blocking`, as appropriate, when it receives the specified `TaskSpec` variant. The background task **should NOT** attempt to modify any app state or render anything to the terminal.

## Launching the Background Task

With the background task defined, the `tui` module needs to be updated to launch the process in the background. This is done by sending the `TaskSpec` we just defined to the `background::manager` using the given `Sender` in the `App.run` function.

## Updating App State and Reporting Progress Updates

Progress updates and app state mutations should not be done directly on background tasks in the interest of thread safety. Instead a `TuiEvent` variant should be defined for each type of event we want to process — one per state modification type (e.g. updating the state with fresh data from an API) and one per progress update type (e.g. updating a progress bar, displaying a toast message). The `App.run` function (or one of the functions it calls) should then process the `TuiEvent` and perform the necessary actions. By having a dedicated thread/task to modify app state or render widgets to the terminal, we can guarantee thread safety without having to rely on `Mutex` or `Arc`.

