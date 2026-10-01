import { CheckCircle2, XCircle } from "lucide-react";
import { withDois } from "@/lib/projects/photonoxide/validation";

/**
 * Rows of the validation report (lib/projects/photonoxide/validation.js), as
 * a table: what was checked, against what (DOIs linked), and the numbers.
 * @param {{ cases: import("@/lib/projects/photonoxide/validation").Case[] }} props
 */
export default function ValidationRows({ cases }) {
  return (
    <div className="mt-4 overflow-x-auto rounded-2xl border border-base-content/10">
      <table className="table table-sm">
        <thead>
          <tr>
            <th>Case</th>
            <th>What, and against what</th>
            <th className="text-right">Measured</th>
            <th className="text-right">Expected</th>
            <th className="text-right">Tolerance</th>
            <th>
              <span className="sr-only">Result</span>
            </th>
          </tr>
        </thead>
        <tbody>
          {cases.map((c) => (
            <tr key={c.id} className="align-top">
              <td>
                <code className="whitespace-nowrap text-xs">{c.id}</code>
                <span className="proj-tag mt-1 block w-fit text-[0.65rem]">{c.tier}</span>
              </td>
              <td className="min-w-[18rem] text-sm">
                <p>{c.what}</p>
                <p className="mt-1 text-base-content/55 text-xs">
                  {withDois(c.against).map((part, i) =>
                    part.doi ? (
                      // biome-ignore lint/suspicious/noArrayIndexKey: the pieces of one string, in order
                      <a key={i} href={`https://doi.org/${part.doi}`} target="_blank" rel="noopener noreferrer" className="link">
                        doi:{part.text}
                      </a>
                    ) : (
                      // biome-ignore lint/suspicious/noArrayIndexKey: as above
                      <span key={i}>{part.text}</span>
                    ),
                  )}
                </p>
              </td>
              <td className="whitespace-nowrap text-right font-mono text-xs">{c.measured}</td>
              <td className="whitespace-nowrap text-right font-mono text-xs">{c.expected}</td>
              <td className="whitespace-nowrap text-right font-mono text-xs">{c.tolerance}</td>
              <td>
                {c.passed ? (
                  <CheckCircle2 size={16} className="text-success" aria-label="passes" />
                ) : (
                  <XCircle size={16} className="text-error" aria-label="fails" />
                )}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
