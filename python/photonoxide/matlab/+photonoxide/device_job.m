function text = device_job(kind, options)
%DEVICE_JOB  The job file that computes a device's design.
%   text = photonoxide.device_job(kind) is the job, as JSON text, that computes the device kind
%   at its defaults by its cheapest model over its own band, as the device's Simulate tab in
%   the photonoxide studio writes it. photonoxide.run_job(text) runs it.
%
%   Options: values, fidelity, stack and compact, as photonoxide.device_spectrum takes them; name (the
%   kind's by default); format ("json", "toml" or "yaml"), the text to save as a job file.
arguments
    kind (1,1) string
    options.values (1,1) struct = struct()
    options.fidelity (1,1) string = ""
    options.stack (1,1) string = ""
    options.name (1,1) string = ""
    options.format (1,1) string = "json"
end
text = string(call(@() py.photonoxide.matlab.device_job(kind, jsonencode(options.values), ...
    options.fidelity, options.stack, options.name, options.format, options.compact)));
end
