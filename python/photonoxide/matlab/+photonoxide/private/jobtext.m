function text = jobtext(j)
%JOBTEXT  A job as the Python side takes it: a job file's path, or a struct with a job file's
%   fields (name, timeout_minutes, task, ...) as JSON text.
if isstruct(j)
    text = jsonencode(j);
else
    text = char(j);
end
end
