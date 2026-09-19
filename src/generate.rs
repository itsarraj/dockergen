use crate::detect::Stack;

/// Renders a working multi-stage Dockerfile for `stack`. `package_name`
/// (Rust only, from `Cargo.toml`) fills in the actual binary name in the
/// final `COPY --from=build` line when known; falls back to a clearly
/// marked placeholder rather than guessing wrong.
pub fn generate_dockerfile(stack: Stack, package_name: Option<&str>) -> String {
    match stack {
        Stack::Rust => rust_dockerfile(package_name.unwrap_or("REPLACE_WITH_YOUR_BINARY_NAME")),
        Stack::NodeNpm => {
            node_dockerfile("npm install", "npm run build --if-present", "npm", "start")
        }
        Stack::NodeYarn => node_dockerfile(
            "yarn install --frozen-lockfile",
            "yarn build --if-present",
            "yarn",
            "start",
        ),
        Stack::NodePnpm => node_dockerfile(
            "pnpm install --frozen-lockfile",
            "pnpm run build --if-present",
            "pnpm",
            "start",
        ),
        Stack::NodeBun => node_dockerfile(
            "bun install --frozen-lockfile",
            "bun run build --if-present",
            "bun",
            "start",
        ),
        Stack::Python => python_dockerfile(),
        Stack::Go => go_dockerfile(),
    }
}

fn rust_dockerfile(binary_name: &str) -> String {
    format!(
        r#"# syntax=docker/dockerfile:1
FROM rust:1-slim AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=build /app/target/release/{binary_name} ./app
EXPOSE 8080
CMD ["./app"]
"#
    )
}

fn node_dockerfile(install_cmd: &str, build_cmd: &str, runner: &str, start_script: &str) -> String {
    format!(
        r#"# syntax=docker/dockerfile:1
FROM node:20-slim AS build
WORKDIR /app
COPY package.json ./
COPY . .
RUN {install_cmd}
RUN {build_cmd}

FROM node:20-slim
WORKDIR /app
COPY --from=build /app .
EXPOSE 3000
CMD ["{runner}", "{start_script}"]
"#
    )
}

fn python_dockerfile() -> String {
    r#"# syntax=docker/dockerfile:1
FROM python:3.12-slim
WORKDIR /app
COPY . .
RUN if [ -f requirements.txt ]; then pip install --no-cache-dir -r requirements.txt; \
    elif [ -f pyproject.toml ]; then pip install --no-cache-dir .; fi
EXPOSE 8000
# REPLACE with your actual entrypoint — this can't be inferred reliably.
CMD ["python", "app.py"]
"#
    .to_string()
}

fn go_dockerfile() -> String {
    r#"# syntax=docker/dockerfile:1
FROM golang:1-alpine AS build
WORKDIR /app
COPY go.mod ./
COPY go.sum* ./
RUN go mod download
COPY . .
RUN CGO_ENABLED=0 go build -o /app/bin ./...

FROM alpine:latest
RUN apk add --no-cache ca-certificates
COPY --from=build /app/bin /bin/app
EXPOSE 8080
CMD ["/bin/app"]
"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_dockerfile_is_multistage_and_uses_the_real_binary_name() {
        let text = generate_dockerfile(Stack::Rust, Some("pgqueue"));
        assert!(text.contains("FROM rust:1-slim AS build"));
        assert!(text.contains("COPY --from=build /app/target/release/pgqueue ./app"));
        assert!(
            text.contains("FROM debian:bookworm-slim"),
            "must have a separate, smaller runtime stage"
        );
    }

    #[test]
    fn rust_dockerfile_without_a_known_name_uses_a_visible_placeholder() {
        let text = generate_dockerfile(Stack::Rust, None);
        assert!(text.contains("REPLACE_WITH_YOUR_BINARY_NAME"));
    }

    #[test]
    fn node_variants_use_the_correct_install_command_each() {
        assert!(generate_dockerfile(Stack::NodeBun, None).contains("bun install"));
        assert!(generate_dockerfile(Stack::NodePnpm, None).contains("pnpm install"));
        assert!(generate_dockerfile(Stack::NodeYarn, None).contains("yarn install"));
        assert!(generate_dockerfile(Stack::NodeNpm, None).contains("npm install"));
    }

    #[test]
    fn python_dockerfile_handles_either_dependency_file() {
        let text = generate_dockerfile(Stack::Python, None);
        assert!(text.contains("requirements.txt"));
        assert!(text.contains("pyproject.toml"));
    }

    #[test]
    fn go_dockerfile_is_multistage_with_cgo_disabled_for_a_static_binary() {
        let text = generate_dockerfile(Stack::Go, None);
        assert!(text.contains("FROM golang:1-alpine AS build"));
        assert!(text.contains("CGO_ENABLED=0"));
        assert!(text.contains("FROM alpine:latest"));
    }

    #[test]
    fn every_generated_dockerfile_declares_a_build_syntax_and_a_cmd() {
        for stack in [Stack::Rust, Stack::NodeNpm, Stack::Python, Stack::Go] {
            let text = generate_dockerfile(stack, Some("x"));
            assert!(
                text.starts_with("# syntax=docker/dockerfile:1"),
                "{:?} missing syntax directive",
                stack
            );
            assert!(text.contains("CMD ["), "{:?} missing a CMD", stack);
        }
    }
}
