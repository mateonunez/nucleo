/**
 * Hello plugin — demonstrates the nucleo plugin protocol.
 *
 * The CLI injects environment variables with the configured prefix:
 *   CLI_ENV_PREFIX  — the prefix itself (e.g. "NUCLEO")
 *   <PREFIX>_TOKEN  — auth token (if authenticated)
 *   <PREFIX>_*_URL  — configured service URLs
 */

const PREFIX = process.env.CLI_ENV_PREFIX || "NUCLEO";

function getEnv(key: string): string | undefined {
  return process.env[`${PREFIX}_${key}`];
}

const command = process.argv[2];

switch (command) {
  case "greet": {
    const name = process.argv[3] || "world";
    console.log(JSON.stringify({ message: `Hello, ${name}!`, plugin: "hello" }));
    break;
  }
  case "status": {
    console.log(
      JSON.stringify({
        plugin_dir: process.env[`${PREFIX}_PLUGIN_DIR`] || "(not set)",
        plugin_name: process.env[`${PREFIX}_PLUGIN_NAME`] || "(not set)",
        has_token: !!getEnv("TOKEN"),
        prefix: PREFIX,
      })
    );
    break;
  }
  default:
    console.error(JSON.stringify({ error: { message: `Unknown command: ${command}`, reason: "validationError" } }));
    process.exit(3);
}
