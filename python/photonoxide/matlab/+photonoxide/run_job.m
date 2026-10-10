function run = run_job(job, options)
%RUN_JOB  Runs a photonoxide job, as photonoxide run --headless does.
%   run = photonoxide.run_job(job) runs job, a job file's path (TOML, JSON or YAML) or a struct
%   with a job file's fields (name, timeout_minutes, task), into a new folder under "runs", and
%   returns run.dir (the folder, which the photonoxide program replays), run.events (a cell
%   array, each event a struct with its type, as jsondecode reads it) and run.stopped
%   ("timeout" or "requested" when it ended early, else "").
%
%   Options: runs_dir ("runs") and timeout_s (3600). The run also stops at the job's own
%   timeout_minutes, whichever comes first.
arguments
    job
    options.runs_dir (1,1) string = "runs"
    options.timeout_s (1,1) double = 3600
end
r = struct(call(@() py.photonoxide.matlab.run_job(jobtext(job), options.runs_dir, ...
    options.timeout_s)));
run.dir = string(r.dir);
run.events = cellfun(@(e) jsondecode(char(e)), cell(r.events), 'UniformOutput', false);
run.stopped = string(r.stopped);
end
