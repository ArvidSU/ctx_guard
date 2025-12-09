describe("authentication token handling", () => {
  test("fails when API token is missing", () => {
    const token = process.env.API_TOKEN;

    if (!token) {
      throw new Error(
        "Authentication token missing. Set the API_TOKEN environment variable before running tests."
      );
    }

    expect(token.length).toBeGreaterThan(10);
  });
});

