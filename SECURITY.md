# Security Policy

## Reporting a vulnerability

Report security problems privately through GitHub's
[private vulnerability reporting](https://docs.github.com/en/code-security/security-advisories/guidance-on-reporting-and-writing-information-about-vulnerabilities/privately-reporting-a-security-vulnerability)
on the repository that contains the crate, or by email to the maintainers at
Yayasan Pesantren Cipansor.

Please do not open a public issue for a security problem.

Include the crate and version, the input that triggers the problem, and what
you expected. If the input is a document you cannot share, reduce it to the
smallest input that still shows the problem.

We aim to acknowledge a report within seven days. This is a volunteer project
maintained alongside other work, so please allow reasonable time for a fix.

## Supported versions

Each crate is versioned independently. Only the latest published version of a
crate receives fixes. While the crates are below `1.0`, a fix may be released
as a patch version and may include a breaking change if the alternative is to
leave the problem in place.

## Scope

These crates are libraries that process data you give them. The relevant
question for each is what a hostile *input* can do, since inputs here are
typically documents received from elsewhere: a WSDL pulled from a partner, a
payment file, a spreadsheet, a number typed by a user.

### What the crates do not do

These properties remove whole classes of problem and are worth stating plainly:

- **No network access.** No crate opens a socket, fetches a URL, or contacts a
  registry. Parsing a document never causes a request to leave the process.
- **No filesystem access.** No crate reads or writes a file. A path in an input
  is data, never something that gets opened.
- **No `unsafe`.** Forbidden at the lint level across the workspace, so there is
  no undefined behaviour to reason about.
- **No entity expansion.** `xmlkit` skips a `DOCTYPE` declaration without
  registering the entities it defines, and only the five predefined XML
  entities plus numeric character references are decoded. This makes two classic
  XML attacks structurally impossible rather than merely mitigated:
  - **XXE.** An external entity such as
    `<!ENTITY x SYSTEM "file:///etc/passwd">` is never resolved. The reference
    `&x;` stays in the output as the literal text `&x;`.
  - **Billion laughs.** Entities that reference other entities are not expanded,
    so the amplification does not happen.
- **No code execution.** A formula in `spreadsheet-formula` is parsed and
  evaluated by this crate. It cannot invoke a macro, a shell, or any external
  process, whatever the formula text says.

### Denial of service

A library that parses input from elsewhere can be turned into a denial of
service by an input that makes it exhaust memory or stack. Two limits are in
place:

- **Element nesting is capped at [`xmlkit::MAX_DEPTH`] (128 levels).** The parser
  descends one stack frame per element, so an unbounded document could overflow
  the stack and abort the process. A document exceeding the cap now returns an
  ordinary error. This was a real defect: before the cap, 35 KB of nested
  elements was enough to abort the process.
- **No expansion of any kind**, as described above, so an input cannot be
  amplified into a much larger one.

Not yet in place, and honestly not claimed:

- **No input size limit.** A very large document will use memory proportional to
  its size. If you accept documents from an untrusted source, apply a size limit
  before parsing. The callers in this workspace do not impose one, because the
  right limit depends on the deployment.
- **No fuzzing harness yet.** The parsers are covered by table-driven tests
  including malformed input, but they have not been run under a fuzzer. If you
  can run one, that would be a welcome contribution.

### Cryptographic verification

None of these crates verify a digital signature, and none should be assumed to
establish authenticity.

- `soap-client` reads WSDL and builds SOAP messages. It does **not** implement
  WS-Security: it does not verify a signature, does not check a certificate, and
  does not validate a timestamp. A SOAP service that signs its responses expects
  the caller to check that signature. If you use this crate against such a
  service, verify the signature yourself before trusting the content. Treating a
  parsed response as authentic without doing so is a mistake the crate cannot
  catch for you.
- `iso20022-rs` parses and builds payment messages. It validates structure, not
  origin. A well-formed `pain.001` is not an authorised one.
- `stdnum-rs` validates that a number is *well formed*: the right length, a
  correct check digit. It does not tell you the number exists, belongs to the
  person presenting it, or is currently valid. Use it to reject typos and
  malformed input, never as an identity check on its own.

### Transport

Where a crate produces something to send, the caller owns the transport.
`soap-client` returns a URL, headers and a body for you to POST with the HTTP
client of your choice. It does not choose a TLS setting or follow redirects.
Use HTTPS, and decide deliberately whether a redirect is acceptable, since
following one can send a request body to a host you did not intend.

## Handling of input

- **Validation is not authorisation.** See above; a valid check digit is a
  statement about format only.
- **Errors do not echo secrets.** Parse errors report a byte offset and a short
  description. They do not include the input, so a document containing personal
  data does not leak into a log through an error message.
- **Nothing is written to disk and nothing is logged by these crates.** Where
  you log an error is your decision, and error messages are safe to log.

## Dependency policy

Crates here depend on the standard library only, except where one workspace
crate builds on another. Fewer dependencies means a smaller supply chain to
trust. A proposed new dependency has to clear that bar: it must do something the
standard library cannot, and it must be worth the trust it adds.
