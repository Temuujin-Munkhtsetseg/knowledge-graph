### 
# PORT OF https://github.com/sst/opencode/blob/dev/packages/opencode/src/session/message-v2.ts 
# We need to capture a structured representation of the messages and parts for the opencode logs
# This is so we can derive granular usage metrics for use in the report phase of the pipeline
# ###

from typing import Optional, Union, List, Dict, Any, Literal
from pydantic import BaseModel, Field, ConfigDict, field_validator

class CacheTokens(BaseModel):
    """Cache token counts"""
    read: int
    write: int


class Tokens(BaseModel):
    """Token usage information"""
    input: int
    output: int
    reasoning: int
    cache: CacheTokens


class TimeInfo(BaseModel):
    """Time information for operations"""
    start: int
    end: Optional[int] = None


class MessageTime(BaseModel):
    """Message timing information"""
    created: int
    completed: Optional[int] = None


class PathInfo(BaseModel):
    """Path information for the message context"""
    cwd: str
    root: str


class ToolStatePending(BaseModel):
    """Tool state when pending"""
    status: Literal["pending"]


class ToolStateRunning(BaseModel):
    """Tool state when running"""
    status: Literal["running"]
    input: Dict[str, Any]
    title: Optional[str] = None
    metadata: Optional[Dict[str, Any]] = None
    time: Dict[str, int]  # Contains 'start' field


class ToolStateCompleted(BaseModel):
    """Tool state when completed"""
    status: Literal["completed"]
    input: Dict[str, Any]
    output: str
    title: Optional[str] = None
    metadata: Optional[Dict[str, Any]] = None
    time: Dict[str, int]  # Contains 'start' and 'end' fields


class ToolStateError(BaseModel):
    """Tool state when error occurred"""
    status: Literal["error"]
    input: Dict[str, Any]
    error: str
    metadata: Optional[Dict[str, Any]] = None
    time: Dict[str, int]  # Contains 'start' and 'end' fields


ToolState = Union[ToolStatePending, ToolStateRunning, ToolStateCompleted, ToolStateError]


class PartBase(BaseModel):
    """Base class for all message parts"""
    model_config = ConfigDict(populate_by_name=True)
    
    id: str
    session_id: Optional[str] = Field(alias="sessionID", default=None)
    message_id: Optional[str] = Field(alias="messageID", default=None)


class TextPart(PartBase):
    """Text content part"""
    type: Literal["text"]
    text: str
    synthetic: Optional[bool] = None
    time: Optional[TimeInfo] = None


class ReasoningPart(PartBase):
    """Reasoning content part"""
    type: Literal["reasoning"]
    text: str
    metadata: Optional[Dict[str, Any]] = None
    time: TimeInfo


class ToolPart(PartBase):
    """Tool invocation part"""
    type: Literal["tool"]
    call_id: str = Field(alias="callID")
    tool: str
    state: ToolState


class StepStartPart(PartBase):
    """Step start marker part"""
    type: Literal["step-start"]


class StepFinishPart(PartBase):
    """Step finish marker part with metrics"""
    type: Literal["step-finish"]
    cost: float
    tokens: Tokens


class FilePartSourceText(BaseModel):
    """Text content within a file source"""
    value: str
    start: int
    end: int


class LSPRange(BaseModel):
    """LSP Range information"""
    start: Dict[str, int]  # line and character
    end: Dict[str, int]    # line and character


class FileSource(BaseModel):
    """File-based source information"""
    type: Literal["file"]
    path: str
    text: FilePartSourceText


class SymbolSource(BaseModel):
    """Symbol-based source information"""
    type: Literal["symbol"]
    path: str
    range: LSPRange
    name: str
    kind: int
    text: FilePartSourceText


FilePartSource = Union[FileSource, SymbolSource]


class FilePart(PartBase):
    """File attachment part"""
    type: Literal["file"]
    mime: str
    filename: Optional[str] = None
    url: str
    source: Optional[FilePartSource] = None


class AgentPart(PartBase):
    """Agent-generated content part"""
    type: Literal["agent"]
    name: str
    source: Optional[FilePartSourceText] = None


class SnapshotPart(PartBase):
    """Snapshot part"""
    type: Literal["snapshot"]
    snapshot: str


class PatchPart(PartBase):
    """Patch part"""
    type: Literal["patch"]
    hash: str
    files: List[str]


MessagePart = Union[
    TextPart,
    ReasoningPart,
    ToolPart,
    StepStartPart,
    StepFinishPart,
    FilePart,
    AgentPart,
    SnapshotPart,
    PatchPart,
]


class AuthError(BaseModel):
    """Authentication error"""
    name: Literal["ProviderAuthError"]
    data: Dict[str, str]  # Contains providerID and message


class OutputLengthError(BaseModel):
    """Output length error"""
    name: Literal["MessageOutputLengthError"]
    data: Dict[str, Any]


class AbortedError(BaseModel):
    """Aborted operation error"""
    name: Literal["MessageAbortedError"]
    data: Dict[str, Any]


class UnknownError(BaseModel):
    """Unknown error"""
    name: str
    data: Dict[str, Any]


MessageError = Union[AuthError, OutputLengthError, AbortedError, UnknownError]


class UserMessage(BaseModel):
    """User message information"""
    model_config = ConfigDict(populate_by_name=True)
    
    id: str
    role: Literal["user"]
    session_id: str = Field(alias="sessionID")
    time: MessageTime


class AssistantMessage(BaseModel):
    """Assistant message information"""
    model_config = ConfigDict(populate_by_name=True)
    
    id: str
    role: Literal["assistant"]
    session_id: str = Field(alias="sessionID")
    time: MessageTime
    error: Optional[MessageError] = None
    system: List[str]
    model_id: str = Field(alias="modelID")
    provider_id: str = Field(alias="providerID")
    mode: str
    path: PathInfo
    summary: Optional[bool] = None
    cost: float
    tokens: Tokens


MessageInfo = Union[UserMessage, AssistantMessage]

# The messages array contains both message objects AND part objects
MessageOrPart = Union[UserMessage, AssistantMessage, MessagePart]

def parse_message_or_part(item: Dict[str, Any]) -> MessageOrPart:
    """
    Parse a dictionary into the appropriate MessageOrPart model.
    
    Args:
        item: Dictionary containing message or part data
        
    Returns:
        MessageOrPart: Parsed Pydantic model or original dict if parsing fails
    """
    if not isinstance(item, dict):
        return item
        
    # Determine type based on presence of 'role' vs 'type' field
    if 'role' in item:
        # This is a message object
        if item['role'] == 'user':
            return UserMessage.model_validate(item)
        elif item['role'] == 'assistant':
            return AssistantMessage.model_validate(item)
        else:
            return item  # Unknown message type
    elif 'type' in item:
        # This is a message part
        part_type = item['type']
        try:
            if part_type == 'text':
                return TextPart.model_validate(item)
            elif part_type == 'reasoning':
                return ReasoningPart.model_validate(item)
            elif part_type == 'tool':
                return ToolPart.model_validate(item)
            elif part_type == 'step-start':
                return StepStartPart.model_validate(item)
            elif part_type == 'step-finish':
                return StepFinishPart.model_validate(item)
            elif part_type == 'file':
                return FilePart.model_validate(item)
            elif part_type == 'agent':
                return AgentPart.model_validate(item)
            elif part_type == 'snapshot':
                return SnapshotPart.model_validate(item)
            elif part_type == 'patch':
                return PatchPart.model_validate(item)
            else:
                print(f"Unknown part type: {part_type}")
                # Unknown part type - keep as raw dict
                return item
        except Exception as e:
            import traceback
            traceback.print_exc()
            # If parsing fails, keep as raw dict
            raise e
    else:
        # No role or type field - keep as raw dict
        raise ValueError(f"No role or type field in item: {item}")

def extract_assistant_messages(messages: List[MessageOrPart]) -> List[AssistantMessage]:
    assistant_messages = []
    for item in messages:
        if isinstance(item, AssistantMessage):
            assistant_messages.append(item)
    return assistant_messages


def extract_user_messages(messages: List[MessageOrPart]) -> List[UserMessage]:
    user_messages = []
    for item in messages:
        if isinstance(item, UserMessage):
            user_messages.append(item)
    return user_messages


def extract_parts(messages: List[MessageOrPart]) -> List[MessagePart]:
    parts = []
    for item in messages:
        if not isinstance(item, (AssistantMessage, UserMessage)):
            parts.append(item)
    return parts

class SessionData(BaseModel):
    """Complete session metrics structure"""
    model_config = ConfigDict(populate_by_name=True)
    
    session_id: str
    messages: List[MessageOrPart]  # Raw dict data that will be parsed
    
    @field_validator('messages')
    @classmethod
    def parse_messages(cls, v):
        """Custom parser for the mixed messages array"""
        if 'messages' not in v:
            return v
            
        parsed_messages = []
        for item in v:
            if not isinstance(item, dict):
                parsed_messages.append(item)
                continue 
            parsed_messages.append(parse_message_or_part(item))
        return parsed_messages
