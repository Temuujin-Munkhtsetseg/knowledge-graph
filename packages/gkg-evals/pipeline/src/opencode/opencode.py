import os
import re
import time
import subprocess
import json
from typing import List, TextIO
from dataclasses import dataclass, field

from src.harness.swe_bench import SweBenchFixtureMetadata, SweBenchPatch, SweBenchCorrectPatch
from src.steps.download import rollback_worktree
from src.opencode.models import SessionData, MessageOrPart
from src.opencode.models import parse_message_or_part
from src.utils import TomlConfig, TomlOpencodeConfig

from src.opencode.constants import OPENCODE_TIME_ELAPSED_DEBOUNCE, OPENCODE_MUTATING_TOOLS, OPENCODE_ALL_DEFAULT_TOOLS, OPENCODE_VERSION
from src.constants import OPENCODE_LOGS_PATH, OPENCODE_MESSAGES_PATH, OPENCODE_MESSAGE_PARTS_PATH

# MultiSweBench
# from harness.multi_swe_bench import MultiSweBenchPatch, MultiSweBenchFixtureMetadata
# from opencode.prompts import OPENCODE_DEFAULT_MULTISWEBENCH_AGENT_DESCRIPTION, OPENCODE_DEFAULT_MULTISWEBENCH_PROMPT

def opencode_config_to_json(config: TomlOpencodeConfig):
    mcp_tools = {tool : True for tool in config.mcp.tools} if config.mcp.enabled else {}
    tool_dict = {tool: True for tool in set(config.tools)} | mcp_tools
    disabled_tools = OPENCODE_ALL_DEFAULT_TOOLS - set(config.tools)
    tool_dict = tool_dict | {tool: False for tool in disabled_tools}
    tool_dict = tool_dict | {"task": False} # Task tool is a distraction in evals
    permission_dict = {tool: "allow" for tool in set(config.tools) if tool in OPENCODE_MUTATING_TOOLS and tool not in disabled_tools}
    permission_dict = permission_dict | {tool: "deny" for tool in disabled_tools if tool in OPENCODE_MUTATING_TOOLS}
    return json.dumps(
        {
            "$schema": "https://opencode.ai/config.json",
            "agent": {
                "build": {
                    "description": config.agent_description,
                    "mode": "primary",
                    "model": config.model,
                    "prompt": config.agent_prompt,
                    "tools": tool_dict,
                    "max_tokens": config.max_tokens,
                },
            },
            "permission": permission_dict,
            "mcp": config.mcp.to_dict(config.mcp.server_name),
            "lsp": {lsp.language: {"disabled": lsp.disabled} for lsp in config.lsp},
        },
        indent=4,
    )

@dataclass
class OpencodeRunSessionData:
    session_id: str
    fixture: SweBenchFixtureMetadata
    patch : SweBenchPatch
    messages: list[MessageOrPart]
    killed: bool = False
    killed_reason: str = ""
    reference_patch: SweBenchCorrectPatch = None
    
    ## This is used for cross run analysis
    file_access_order: list[str] = field(default_factory=list)
    patch_paths: list[str] = field(default_factory=list)

    def to_dict(self):
        serialized_messages = []
        for message in self.messages:
            if hasattr(message, 'model_dump'):
                serialized_messages.append(message.model_dump())
            else:
                serialized_messages.append(message)
        return {
            "session_id": self.session_id, 
            "fixture": self.fixture.to_dict(), 
            "patch": self.patch.to_dict(), 
            "messages": serialized_messages,
            "killed": self.killed,
            "killed_reason": self.killed_reason
        }

    @classmethod
    def from_dict(cls, d: dict):
        try:
            raw_messages = d.get("messages", [])
            parsed_messages = [parse_message_or_part(item) for item in raw_messages]
            patch = SweBenchPatch.from_dict(d.get("patch"))
            fixture = SweBenchFixtureMetadata.from_dict(d.get("fixture"))

            return cls(
                session_id=d.get("session_id"), 
                fixture=fixture, 
                patch=patch, 
                messages=parsed_messages,
                killed=d.get("killed"),
                killed_reason=d.get("killed_reason")
            )
        except Exception as e:
            import traceback
            traceback.print_exc()
            raise e

@dataclass
class OpencodeRunSubprocessResult:
    session_id: str
    killed: bool
    killed_reason: str

class Opencode:
    def __init__(self, toml_config: TomlConfig):
        self.logs_stdout = toml_config.pipeline.opencode_logs_stdout
        self.toml_config = toml_config
        self.setup_opencode_executable()
        self.setup_config()

    def setup_config(self):
        opencode_config = opencode_config_to_json(self.toml_config.opencode)
        opencode_config_path = self.toml_config.pipeline.session_paths.opencode_config_path
        with open(opencode_config_path, "w") as f:
            print(f"Writing config to {opencode_config_path}")
            print(opencode_config)
            f.write(opencode_config)

    def setup_opencode_executable(self):
        opencode_version = f"opencode-ai@{OPENCODE_VERSION}"
        args = ["npx", '--yes', opencode_version, "--version"]

        # run once to install the executable
        subprocess.run(args)

        # check again if the executable is available
        p = subprocess.run(args, capture_output=True)
        if p.returncode != 0:
            raise ValueError("Failed to get opencode version")
        stdout = p.stdout.decode("utf-8").strip()
        print(f"Found opencode-ai: {stdout}")
        if stdout != OPENCODE_VERSION:
            raise ValueError(f"Failed to install opencode-ai: {stdout} != {OPENCODE_VERSION}")

    def capture_session_id(self, line: str) -> str:
        pattern = r'INFO.*service=session id=([^\s]+).*created'
        match = re.search(pattern, line)
        if match:
            return match.group(1).strip()
        return None

    def run_subprocess(self, fixture: SweBenchFixtureMetadata, user_prompt: str, logs_fd: TextIO) -> OpencodeRunSubprocessResult:
        # Make sure opencode uses the correct config file + run the agent
        # Note: auth here is setup implicitly bc it's in os.environ
        opencode_env = os.environ.copy()
        opencode_config_path = self.toml_config.pipeline.session_paths.opencode_config_path
        opencode_env["OPENCODE_CONFIG"] = str(opencode_config_path)
        
        print(f"Running OpenCode with command: opencode run --print-logs --log-level INFO '{user_prompt}'")
        print(f"Working directory: {fixture.worktree_path}")
        print("=" * 80)

        args =[
            "npx",
            f"opencode-ai@{OPENCODE_VERSION}",
            "run",
            "--print-logs",
            "--log-level",
            "INFO",
            user_prompt
        ]

        start_time = time.time()
        session_id = None
        last_time_print = 0  # Track when we last printed elapsed time
        killed_by_timeout = False 

        # Use Popen to stream output in real-time while capturing for logs
        with subprocess.Popen(
            args,
            cwd=fixture.worktree_path,
            env=opencode_env,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,  # Merge stderr into stdout
            text=True,
            bufsize=1,  # Line buffered
            universal_newlines=True
        ) as process:
            # Stream output line by line
            for line in iter(process.stdout.readline, ''):
                time_elapsed = time.time() - start_time
                if not session_id:
                    session_id = self.capture_session_id(line)
                    if session_id:
                        print(f"Captured Opencode Session ID: {session_id}")
                
                # Print elapsed time every 5 seconds
                if time_elapsed - last_time_print >= OPENCODE_TIME_ELAPSED_DEBOUNCE and self.logs_stdout:
                    print(f"[TIME] Elapsed: {time_elapsed:.1f} seconds")
                    last_time_print = time_elapsed
                
                if line.strip().startswith("INFO"):
                    continue
                if self.logs_stdout:
                    print(line, end='')  # Print to stdout in real-time
                if time_elapsed > self.toml_config.pipeline.fixture_timeout:
                    process.kill()
                    print(f"OpenCode killed after {time_elapsed} seconds, due to timeout")
                    killed_by_timeout = True
                    break
                logs_fd.write(line)
            
            # Wait for process to complete
            process.wait()
            
        print("=" * 80)
        print(f"[TIME] Total elapsed: {time.time() - start_time} seconds")
        print(f"OpenCode exit code: {process.returncode}")

        if process.returncode != 0:
            print(f"WARNING: OpenCode failed with exit code {process.returncode}")

        killed = killed_by_timeout or process.returncode != 0
        killed_reason = "timeout" if killed_by_timeout else "error" if process.returncode != 0 else ""

        return OpencodeRunSubprocessResult(
            session_id=session_id,
            killed=killed,
            killed_reason=killed_reason
        )


    def run(self, fixture: SweBenchFixtureMetadata) -> OpencodeRunSessionData:
        # From MultiSweBench
        # if len(fixture.resolved_issues) == 0:
        #     raise ValueError("No issues to fix")
        # issue = fixture.resolved_issues[0]

        # Rollback the worktree to the original state just in case
        rollback_worktree(fixture)

        # From MultiSweBench
        # user_prompt = """
        # Address the following Github issue for the codebase:
        # <issue>
        # <title>
        # {issue.title}
        # </title>
        # <description>
        # {issue.body}
        # </issue>
        # """.format(issue=issue)

        # SWE BENCH SPECIFIC PROMPT
        user_prompt = self.toml_config.opencode.user_prompt.format(
            problem_statement=fixture.problem_statement)

        print(f"Running opencode for {fixture.org}/{fixture.repo}#{fixture.base_commit}")
        print(f"Working directory: {fixture.worktree_path}")
        print(f"Command: opencode run '{user_prompt[:100]}...'")

        # Setup logs dir
        opencode_logs_dir = self.toml_config.pipeline.session_dir / "agent_logs" / fixture.instance_id
        opencode_logs_dir.mkdir(parents=True, exist_ok=True)
        opencode_logs_path = opencode_logs_dir / OPENCODE_LOGS_PATH
        logs_fd = open(opencode_logs_path, "w")
        
        # Run the opencode agent, stream logs to the file descriptor
        run_result = self.run_subprocess(fixture, user_prompt, logs_fd)
        logs_fd.flush()
        logs_fd.close()

        # get git diff from the worktree directory
        git_diff = subprocess.run(["git", "diff", "HEAD"], cwd=fixture.worktree_path, capture_output=True)

        if git_diff.returncode != 0:
            raise ValueError("Failed to get git diff")

        # Then rollback the changes (both modified and new files)
        rollback_worktree(fixture)

        if run_result.session_id is None:
            raise ValueError("Session ID not found")

        messages = self.session_messages(run_result.session_id)
        patch = SweBenchPatch(
            instance_id=fixture.instance_id,
            model_patch=git_diff.stdout.decode("utf-8"),
            model_name_or_path=self.toml_config.opencode.model,
        )

        session_data = OpencodeRunSessionData(
            session_id=run_result.session_id,
            fixture=fixture,
            patch=patch,
            messages=messages,
            killed=run_result.killed,
            killed_reason=run_result.killed_reason
        )
        print("finished opencode")
        return session_data

    def session_messages(self, session_id: str) -> List[MessageOrPart]:
        messages_dir = OPENCODE_MESSAGES_PATH / f"{session_id}"
        messages = []
        for file in messages_dir.iterdir():
            if file.is_file():
                with open(file, "r") as f:
                    message = json.load(f)
                    messages.append(message)
                    message_id = message["id"]
                    message_parts_dir = OPENCODE_MESSAGE_PARTS_PATH / f"{message_id}"
                    for part_file in message_parts_dir.iterdir():
                        if part_file.is_file():
                            with open(part_file, "r") as f:
                                part = json.load(f)
                                messages.append(part)

        session_data = SessionData.model_validate({
            "session_id": session_id,
            "messages": messages
        })
        return session_data.messages
