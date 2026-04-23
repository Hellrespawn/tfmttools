const { html, render } = await import(globalThis.__tfmtReportVendorUrl);

const statusText = status => status.replaceAll("_", " ");

function durationText(ms) {
    if (ms >= 1000) return `${(ms / 1000).toFixed(2)} s`;
    return `${ms} ms`;
}

function dateText(value) {
    const date = new Date(value);
    if (Number.isNaN(date.valueOf())) return value;
    return date.toLocaleString(undefined, {
        dateStyle: "medium",
        timeStyle: "medium",
    });
}

function pathName(path) {
    if (!path) return "";
    const parts = path.split("/");
    return parts.at(-1) || path;
}

function compactPath(path) {
    if (!path) return "";
    const marker = "/input/";
    const markerIndex = path.indexOf(marker);
    if (markerIndex >= 0) return `input/${path.slice(markerIndex + marker.length)}`;

    const parts = path.split("/");
    if (parts.length <= 4) return path;
    return `.../${parts.slice(-3).join("/")}`;
}

function commandLine(args) {
    return args.map(shellArg).join(" ");
}

function shellArg(arg) {
    if (!arg) return "''";
    if (!/[\s'"\\]/.test(arg)) return arg;
    return `'${arg.replaceAll("'", "'\\''")}'`;
}

function pluralize(count, singular, plural = `${singular}s`) {
    return count === 1 ? singular : plural;
}

function firstLine(value) {
    return value?.trim().split(/\r?\n/).find(Boolean) ?? "";
}

function failureTextPrefix(count, noun) {
    return `${count} failing ${pluralize(count, noun)}`;
}

function skipReasonText(reason) {
    if (reason === "previous_step_failed") {
        return "Skipped after an earlier step failed.";
    }

    return reason ? statusText(reason) : "No skip reason recorded.";
}

function expectationDetails(outcome) {
    const [kind, value] = Object.entries(outcome)[0] ?? ["unknown", ""];

    if (kind === "checksum_mismatch") {
        return {
            status: "failed",
            label: "Checksum mismatch",
            path: value.path,
            detail: html`
                <dl class="checksum">
                    <div>
                        <dt>Expected</dt>
                        <dd><code>${value.expected}</code></dd>
                    </div>
                    <div>
                        <dt>Actual</dt>
                        <dd><code>${value.actual}</code></dd>
                    </div>
                </dl>
            `,
        };
    }

    if (kind === "not_present") {
        return {
            status: "failed",
            label: "Missing file",
            path: value,
            detail: null,
        };
    }

    if (kind === "ok") {
        return {
            status: "passed",
            label: "Found",
            path: value,
            detail: null,
        };
    }

    return {
        status: "failed",
        label: statusText(kind),
        path: String(value),
        detail: null,
    };
}

function expectationFailureSummary(expectations) {
    if (!expectations || expectations.status === "passed") return null;

    const remaining = expectations.files_remaining_after_previous ?? [];
    if (remaining.length > 0) {
        return {
            label: failureTextPrefix(remaining.length, "file"),
            message: `Still present from previous state: ${compactPath(remaining[0])}`,
        };
    }

    const failedOutcomes = expectations.outcomes
        .map(expectationDetails)
        .filter(outcome => outcome.status !== "passed");

    if (failedOutcomes.length === 0) {
        return {
            label: "Expectation failed",
            message: "The expectation group did not pass.",
        };
    }

    const firstFailure = failedOutcomes[0];
    const path = firstFailure.path ? `: ${compactPath(firstFailure.path)}` : "";

    return {
        label: failureTextPrefix(failedOutcomes.length, "expectation"),
        message: `${firstFailure.label}${path}`,
    };
}

function commandFailureSummary(command) {
    if (!command || command.status === "passed") return null;

    const exit = command.exit_code == null ? "signal" : command.exit_code;
    const stderr = firstLine(command.stderr);

    return {
        label: "Command failed",
        message: stderr ? `exit ${exit}: ${stderr}` : `exit ${exit}`,
    };
}

function stepFailureSummary(step) {
    if (step.status === "passed") return null;

    if (step.status === "skipped") {
        return {
            label: "Step skipped",
            message: skipReasonText(step.skip_reason),
            status: "skipped",
        };
    }

    return (
        commandFailureSummary(step.command_outcome) ??
        expectationFailureSummary(step.expectations_outcome) ?? {
            label: statusText(step.status),
            message: "No failure detail recorded.",
            status: step.status,
        }
    );
}

function FailureSummary({ summary, compact = false }) {
    if (!summary) return null;

    return html`
        <p
            class=${[
                "failure-summary",
                summary.status ?? "failed",
                compact ? "compact" : "",
            ].join(" ")}
        >
            <strong>${summary.label}</strong>
            <span>${summary.message}</span>
        </p>
    `;
}

function Status({ status }) {
    return html`<span class=${`status ${status}`}>${statusText(status)}</span>`;
}

function SummaryItem({ label, value, status }) {
    return html`
        <div class=${`summary-item ${status ?? ""}`}>
            <span>${label}</span>
            <strong>${value}</strong>
        </div>
    `;
}

function ExitCode({ command }) {
    const isSignal = command.exit_code == null;
    const value = isSignal ? "signal" : command.exit_code;

    return html`
        <span class=${`exit-code ${command.status}`} title="Process exit code">
            exit ${value}
        </span>
    `;
}

function OutputBlock({ title, value }) {
    if (!value) return null;

    return html`
        <details class="output" open=${title === "stderr"}>
            <summary>${title}</summary>
            <pre><code>${value}</code></pre>
        </details>
    `;
}

function CommandOutcome({ command }) {
    if (!command) return null;

    return html`
        <section class="command-detail">
            <div class="command-bar">
                <span class="detail-label">Command</span>
                <${ExitCode} command=${command} />
                <code>${commandLine(command.arguments)}</code>
                <${Status} status=${command.status} />
            </div>
            <${OutputBlock} title="stdout" value=${command.stdout} />
            <${OutputBlock} title="stderr" value=${command.stderr} />
        </section>
    `;
}

function RemainingFiles({ files }) {
    if (!files?.length) return null;

    return html`
        <div class="remaining-files">
            <h5>Files left from previous state</h5>
            <ul>
                ${files.map(
                    file => html`
                        <li title=${file}>
                            <code>${compactPath(file)}</code>
                        </li>
                    `,
                )}
            </ul>
        </div>
    `;
}

function ExpectationRow({ outcome }) {
    const detail = expectationDetails(outcome);

    return html`
        <tr class=${`expectation-row ${detail.status}`}>
            <td><span class="result-dot" aria-hidden="true"></span></td>
            <td><strong>${detail.label}</strong></td>
            <td><code title=${detail.path}>${compactPath(detail.path)}</code></td>
            <td>${detail.detail}</td>
        </tr>
    `;
}

function ExpectationsOutcome({ expectations }) {
    if (!expectations) return null;

    return html`
        <section class="expectations-detail">
            <div class="expectations-heading">
                <span class="detail-label">Expectations</span>
                <span>${expectations.outcomes.length} checks</span>
                <${Status} status=${expectations.status} />
            </div>
            <${RemainingFiles}
                files=${expectations.files_remaining_after_previous}
            />
            <table class="expectation-table">
                <thead>
                    <tr>
                        <th></th>
                        <th>Result</th>
                        <th>Path</th>
                        <th>Details</th>
                    </tr>
                </thead>
                <tbody>
                    ${expectations.outcomes.map(
                        outcome =>
                            html`<${ExpectationRow} outcome=${outcome} />`,
                    )}
                </tbody>
            </table>
        </section>
    `;
}

function Step({ step }) {
    if (step.status === "skipped") {
        const summary = stepFailureSummary(step);

        return html`
            <section class="step skipped">
                <div class="step-header static-step-header">
                    <div class="step-summary-row">
                        <span
                            class="fold-toggle step-toggle empty-toggle"
                            aria-hidden="true"
                        ></span>
                        <div>
                            <span class="step-label">Test step</span>
                            <h3>${step.name}</h3>
                            <${FailureSummary}
                                summary=${summary}
                                compact=${true}
                            />
                        </div>
                    </div>
                    <div class="step-meta">
                        <span>${durationText(step.duration_ms)}</span>
                        <${Status} status=${step.status} />
                    </div>
                </div>
            </section>
        `;
    }

    const open = step.status !== "passed";
    const summary = stepFailureSummary(step);

    return html`
        <details class=${`step ${step.status}`} open=${open}>
            <summary class="step-header">
                <div class="step-summary-row">
                    <span
                        class="fold-toggle step-toggle"
                        aria-hidden="true"
                    ></span>
                    <div>
                        <span class="step-label">Test step</span>
                        <h3>${step.name}</h3>
                        <${FailureSummary} summary=${summary} compact=${true} />
                    </div>
                </div>
                <div class="step-meta">
                    <span>${durationText(step.duration_ms)}</span>
                    <${Status} status=${step.status} />
                </div>
            </summary>
            <div class="step-body">
                <${CommandOutcome} command=${step.command_outcome} />
                <${ExpectationsOutcome}
                    expectations=${step.expectations_outcome}
                />
            </div>
        </details>
    `;
}

function Case({ testCase }) {
    const failedSteps = testCase.steps.filter(step => step.status !== "passed");
    const attentionSteps = failedSteps.filter(
        step => step.status !== "skipped",
    );
    const open = testCase.status !== "passed";
    const firstFailure = failedSteps
        .map(stepFailureSummary)
        .find(summary => summary != null);

    return html`
        <details class=${`case ${testCase.status}`} open=${open}>
            <summary class="case-header">
                <div class="case-header-main">
                    <div class="case-summary-row">
                        <span
                            class="fold-toggle case-toggle"
                            aria-hidden="true"
                        ></span>
                        <div class="case-title">
                            <span class="case-label">Test</span>
                            <h2>${testCase.name}</h2>
                        </div>
                    </div>
                    <div class="case-meta">
                        <span>${durationText(testCase.duration_ms)}</span>
                        <span>${testCase.steps.length} steps</span>
                        ${attentionSteps.length
                            ? html`<span>
                                  ${attentionSteps.length} need attention
                              </span>`
                            : null}
                        ${testCase.cli?.work_dir &&
                        html`
                            <code title=${testCase.cli.work_dir}>
                                ${pathName(testCase.cli.work_dir)}
                            </code>
                        `}
                        <${Status} status=${testCase.status} />
                    </div>
                </div>
                <p>${testCase.description}</p>
                <${FailureSummary} summary=${firstFailure} />
            </summary>
            <div class="steps">
                ${testCase.steps.map(step => html`<${Step} step=${step} />`)}
            </div>
        </details>
    `;
}

function RunMeta({ report }) {
    return html`
        <dl class="run-meta">
            <div>
                <dt>Started</dt>
                <dd>${dateText(report.started_at)}</dd>
            </div>
            <div>
                <dt>Generated</dt>
                <dd>${dateText(report.generated_at)}</dd>
            </div>
            <div>
                <dt>Duration</dt>
                <dd>${durationText(report.duration_ms)}</dd>
            </div>
            <div>
                <dt>Runner</dt>
                <dd>${report.runner}</dd>
            </div>
        </dl>
    `;
}

function Report({ report }) {
    return html`
        <header class=${`run-header ${report.status}`}>
            <div class="run-title">
                <h1>tfmt test report</h1>
                <${Status} status=${report.status} />
            </div>
            <${RunMeta} report=${report} />
            <div class="summary">
                <${SummaryItem} label="Total" value=${report.summary.total} />
                <${SummaryItem}
                    label="Passed"
                    value=${report.summary.passed}
                    status="passed"
                />
                <${SummaryItem}
                    label="Failed"
                    value=${report.summary.failed}
                    status="failed"
                />
                <${SummaryItem}
                    label="Skipped"
                    value=${report.summary.skipped}
                    status="skipped"
                />
                <${SummaryItem}
                    label="Timed out"
                    value=${report.summary.timed_out}
                    status="timed_out"
                />
            </div>
        </header>
        <section class="cases" aria-label="Test cases">
            ${report.cases.map(
                testCase => html`<${Case} testCase=${testCase} />`,
            )}
        </section>
    `;
}

async function main() {
    const reportJsonFileName =
        globalThis.__tfmtReportJsonFileName ?? "report.json";
    const response = await fetch(reportJsonFileName, { cache: "no-store" });
    if (!response.ok) {
        throw new Error(
            `failed to load ${reportJsonFileName}: ${response.status}`,
        );
    }

    const report = await response.json();
    render(html`<${Report} report=${report} />`, document.querySelector("#app"));
}

main().catch(error => {
    render(
        html`
            <section class="run-header error">
                <h1>Unable to load report</h1>
                <p>${error.message}</p>
            </section>
        `,
        document.querySelector("#app"),
    );
});
