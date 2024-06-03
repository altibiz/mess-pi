# Environment

This document outlines the development environment requirements for this
project. These requirements are necessary to execute the commands defined in the
`justfile`.

## Requirements

- **Rust**: The project uses Rust, and the `cargo` command is used for building,
  testing, and running the Rust code. It's also used for generating
  documentation and formatting the Rust code.
- **Docker**: Docker is used to manage services that the application depends on.
  The `docker compose up -d` command is used to start these services, and
  `docker compose down -v` is used to stop them.

## Optional Requirements

The following tools are optional for some workflows but recommended for
development:

### Probe

- **Python**: Python is used for the `probe` script. You need to have Python
  installed to run this script.
- **Poetry**: Poetry is used for managing Python dependencies.

### Formatting

- **Yapf**: Yapf is used for formatting Python code in the project.
- **Prettier**: Prettier is used for formatting and checking the format of the
  code in the project.
- **shfmt**: shfmt is used for formatting shell scripts in the project.

### Linting

- **ShellCheck**: ShellCheck is used for linting shell scripts.
- **cspell**: cspell is used for spell checking in the project.
- **Ruff**: Ruff is used for checking Rust code in the project.
- **Clippy**: Clippy is a Rust linter that's used in the project.
- **Pyright**: Pyright is used for type checking Python code.

### Documentation

- **mdbook**: mdbook is used for building the documentation.

### Stress testing

- **GNS3**: GNS3 is used to simulated networks.

## Development Workflow

The development workflow is managed by `just`, a command runner that's similar
to `make`. The `justfile` at the root of the repository defines various commands
for building, testing, running, and managing the project.

Here are the steps to set up the development environment and use `just`:

1. **Install Dependencies**: Install all the required tools listed in this
   chapter.

2. **Prepare the Environment**: Run `just prepare` to install Python
   dependencies, start Docker services, and run database migrations.

3. **Run the Application**: Use `just run` to run the application. You can pass
   arguments to the application by appending them to the command, like
   `just run --arg`.

4. **Run the Probe Script**: Use `just probe` to run the probe script. You can
   pass arguments to the script in the same way as the run command.

5. **Format the Code**: Use `just format` to format the code in the project
   using various formatters.

6. **Lint the Code**: Use `just lint` to lint the code in the project using
   various linters.

7. **Test the Code**: Use `just test` to run the tests for the project.

8. **Build the Project**: Use `just build` to build the project. This will
   create a release build of the project and move the output to the `artifacts`
   directory.

9. **Generate Documentation**: Use `just docs` to generate the project's
   documentation. This will build the documentation and move the output to the
   `artifacts` directory.

Remember to run `just prepare` whenever you pull new changes from the
repository, to ensure your environment is up-to-date.
