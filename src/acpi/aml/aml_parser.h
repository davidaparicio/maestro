#ifndef AML_PARSER_H
# define AML_PARSER_H

# include <memory/memory.h>
# include <libc/errno.h>

# define ALIAS_OP		((uint8_t) 0x06)
# define NAME_OP		((uint8_t) 0x08)
# define SCOPE_OP		((uint8_t) 0x10)
# define EXT_OP_PREFIX	((uint8_t) 0x5b)
# define BANK_FIELD_OP	((uint8_t) 0x87)
# define CONTINUE_OP	((uint8_t) 0x9f)
# define IF_OP			((uint8_t) 0xa0)
# define NOOP_OP		((uint8_t) 0xa3)
# define BREAK_OP		((uint8_t) 0xa5)
# define BREAKPOINT_OP	((uint8_t) 0xcc)

# define DUAL_NAME_PREFIX	0x2e
# define MULTI_NAME_PREFIX	0x2f

# define IS_LEAD_NAME_CHAR(c)	(((c) >= 'A' && (c) <= 'Z') || (c) == '_')
# define IS_DIGIT_CHAR(c)		((c) >= '0' && (c) <= '9')
# define IS_NAME_CHAR(c)		(IS_LEAD_NAME_CHAR(c) || IS_DIGIT_CHAR(c))
# define IS_ROOT_CHAR(c)		((c) == '\\')
# define IS_PREFIX_CHAR(c)		((c) == '^')

enum node_type
{
	AML_CODE,
	DEF_BLOCK_HEADER,
	TABLE_SIGNATURE,
	TABLE_LENGTH,
	SPEC_COMPLIANCE,
	CHECK_SUM,
	OEM_ID,
	OEM_TABLE_ID,
	OEM_REVISION,
	CREATOR_ID,
	CREATOR_REVISION,
	ROOT_CHAR,
	NAME_SEG,
	NAME_STRING,
	PREFIX_PATH,
	NAME_PATH,
	DUAL_NAME_PATH,
	MULTI_NAME_PATH,
	SEG_COUNT,
	SIMPLE_NAME,
	SUPER_NAME,
	NULL_NAME,
	TARGET,
	COMPUTATIONAL_DATA,
	DATA_OBJECT,
	DATA_REF_OBJECT,
	BYTE_CONST,
	BYTE_PREFIX,
	WORD_CONST,
	WORD_PREFIX,
	D_WORD_CONST,
	D_WORD_PREFIX,
	Q_WORD_CONST,
	Q_WORD_PREFIX,
	STRING,
	STRING_PREFIX,
	CONST_OBJ,
	BYTE_LIST,
	BYTE_DATA,
	WORD_DATA,
	DWORD_DATA,
	QWORD_DATA,
	ASCII_CHAR_LIST,
	ASCII_CHAR,
	NULL_CHAR,
	ZERO_OP,
	ONE_OP,
	ONES_OP,
	REVISION_OP,
	PKG_LENGTH,
	PKG_LEAD_BYTE,
	OBJECT,
	TERM_OBJ,
	TERM_LIST,
	TERM_ARG,
	METHOD_INVOCATION,
	TERM_ARG_LIST,
	NAME_SPACE_MODIFIER_OBJ,
	DEF_ALIAS,
	DEF_NAME,
	DEF_SCOPE,
	NAMED_OBJ,
	DEF_BANK_FIELD,
	BANK_VALUE,
	FIELD_FLAGS,
	FIELD_LIST,
	NAMED_FIELD,
	RESERVED_FIELD,
	ACCESS_FIELD,
	ACCESS_TYPE,
	ACCESS_ATTRIB,
	CONNECT_FIELD,
	DEF_CREATE_BIT_FIELD,
	CREATE_BIT_FIELD_OP,
	SOURCE_BUFF,
	BIT_INDEX,
	DEF_CREATE_BYTE_FIELD,
	CREATE_BYTE_FIELD_OP,
	BYTE_INDEX,
	DEF_CREATE_D_WORD_FIELD,
	CREATE_D_WORD_FIELD_OP,
	DEF_CREATE_FIELD,
	CREATE_FIELD_OP,
	NUM_BITS,
	DEF_CREATE_Q_WORD_FIELD,
	CREATE_Q_WORD_FIELD_OP,
	DEF_CREATE_WORD_FIELD,
	CREATE_WORD_FIELD_OP,
	DEF_DATA_REGION,
	DATA_REGION_OP,
	DEF_DEVICE,
	DEVICE_OP,
	DEF_EVENT,
	EVENT_OP,
	DEF_EXTERNAL,
	EXTERNAL_OP,
	OBJECT_TYPE,
	ARGUMENT_COUNT,
	DEF_FIELD,
	FIELD_OP,
	DEF_INDEX_FIELD,
	INDEX_FIELD_OP,
	DEF_METHOD,
	METHOD_OP,
	METHOD_FLAGS,
	DEF_MUTEX,
	MUTEX_OP,
	SYNC_FLAGS,
	DEF_OP_REGION,
	OP_REGION_OP,
	REGION_SPACE,
	REGION_OFFSET,
	REGION_LEN,
	DEF_POWER_RES,
	POWER_RES_OP,
	SYSTEM_LEVEL,
	RESOURCE_ORDER,
	DEF_PROCESSOR,
	PROCESSOR_OP,
	PROC_ID,
	PBLK_ADDR,
	PBLK_LEN,
	DEF_THERMAL_ZONE,
	THERMAL_ZONE_OP,
	EXTENDED_ACCESS_FIELD,
	EXTENDED_ACCESS_ATTRIB,
	FIELD_ELEMENT,
	TYPE1_OPCODE,
	DEF_BREAK,
	DEF_BREAK_POINT,
	DEF_CONTINUE,
	DEF_ELSE,
	DEF_FATAL,
	FATAL_OP,
	FATAL_TYPE,
	FATAL_CODE,
	FATAL_ARG,
	DEF_IF_ELSE,
	PREDICATE,
	DEF_LOAD,
	LOAD_OP,
	DDB_HANDLE_OBJECT,
	DEF_NOOP,
	DEF_NOTIFY,
	NOTIFY_OP,
	NOTIFY_OBJECT,
	NOTIFY_VALUE,
	DEF_RELEASE,
	RELEASE_OP,
	MUTEX_OBJECT,
	DEF_RESET,
	RESET_OP,
	EVENT_OBJECT,
	DEF_RETURN,
	RETURN_OP,
	ARG_OBJECT,
	DEF_SIGNAL,
	SIGNAL_OP,
	DEF_SLEEP,
	SLEEP_OP,
	MSEC_TIME,
	DEF_STALL,
	STALL_OP,
	USEC_TIME,
	DEF_WHILE,
	WHILE_OP,
	TYPE2_OPCODE,
	TYPE6_OPCODE,
	DEF_ACQUIRE,
	ACQUIRE_OP,
	TIMEOUT,
	DEF_ADD,
	ADD_OP,
	OPERAND,
	DEF_AND,
	AND_OP,
	DEF_BUFFER,
	BUFFER_OP,
	BUFFER_SIZE,
	DEF_CONCAT,
	CONCAT_OP,
	DATA,
	DEF_CONCAT_RES,
	CONCAT_RES_OP,
	BUF_DATA,
	DEF_COND_REF_OF,
	COND_REF_OF_OP,
	DEF_COPY_OBJECT,
	COPY_OBJECT_OP,
	DEF_DECREMENT,
	DECREMENT_OP,
	DEF_DEREF_OF,
	DEREF_OF_OP,
	OBJ_REFERENCE,
	DEF_DIVIDE,
	DIVIDE_OP,
	DIVIDEND,
	DIVISOR,
	REMAINDER,
	QUOTIENT,
	DEF_FIND_SET_LEFT_BIT,
	FIND_SET_LEFT_BIT_OP,
	DEF_FIND_SET_RIGHT_BIT,
	FIND_SET_RIGHT_BIT_OP,
	DEF_FROM_BCD,
	FROM_BCD_OP,
	BCD_VALUE,
	DEF_INCREMENT,
	INCREMENT_OP,
	DEF_INDEX,
	INDEX_OP,
	BUFF_PKG_STR_OBJ,
	INDEX_VALUE,
	DEF_L_AND,
	LAND_OP,
	DEF_L_EQUAL,
	LEQUAL_OP,
	DEF_L_GREATER,
	LGREATER_OP,
	DEF_L_GREATER_EQUAL,
	LGREATER_EQUAL_OP,
	DEF_L_LESS,
	LLESS_OP,
	DEF_L_LESS_EQUAL,
	LLESS_EQUAL_OP,
	DEF_L_NOT,
	LNOT_OP,
	DEF_L_NOT_EQUAL,
	LNOT_EQUAL_OP,
	DEF_LOAD_TABLE,
	LOAD_TABLE_OP,
	DEF_L_OR,
	LOR_OP,
	DEF_MATCH,
	MATCH_OP,
	SEARCH_PKG,
	MATCH_OPCODE,
	START_INDEX,
	DEF_MID,
	MID_OP,
	MID_OBJ,
	DEF_MOD,
	MOD_OP,
	DEF_MULTIPLY,
	MULTIPLY_OP,
	DEF_N_AND,
	NAND_OP,
	DEF_N_OR,
	NOR_OP,
	DEF_NOT,
	NOT_OP,
	DEF_OBJECT_TYPE,
	OBJECT_TYPE_OP,
	DEF_OR,
	OR_OP,
	DEF_PACKAGE,
	PACKAGE_OP,
	DEF_VAR_PACKAGE,
	VAR_PACKAGE_OP,
	NUM_ELEMENTS,
	VAR_NUM_ELEMENTS,
	PACKAGE_ELEMENT_LIST,
	PACKAGE_ELEMENT,
	DEF_REF_OF,
	REF_OF_OP,
	DEF_SHIFT_LEFT,
	SHIFT_LEFT_OP,
	SHIFT_COUNT,
	DEF_SHIFT_RIGHT,
	SHIFT_RIGHT_OP,
	DEF_SIZE_OF,
	SIZE_OF_OP,
	DEF_STORE,
	STORE_OP,
	DEF_SUBTRACT,
	SUBTRACT_OP,
	DEF_TIMER,
	TIMER_OP,
	DEF_TO_BCD,
	TO_BCD_OP,
	DEF_TO_BUFFER,
	TO_BUFFER_OP,
	DEF_TO_DECIMAL_STRING,
	TO_DECIMAL_STRING_OP,
	DEF_TO_HEX_STRING,
	TO_HEX_STRING_OP,
	DEF_TO_INTEGER,
	TO_INTEGER_OP,
	DEF_TO_STRING,
	LENGTH_ARG,
	TO_STRING_OP,
	DEF_WAIT,
	WAIT_OP,
	DEF_X_OR,
	XOR_OP,
	ARG_OBJ,
	ARG0_OP,
	ARG1_OP,
	ARG2_OP,
	ARG3_OP,
	ARG4_OP,
	ARG5_OP,
	ARG6_OP,
	LOCAL_OBJ,
	LOCAL0_OP,
	LOCAL1_OP,
	LOCAL2_OP,
	LOCAL3_OP,
	LOCAL4_OP,
	LOCAL5_OP,
	LOCAL6_OP,
	LOCAL7_OP,
	DEBUG_OBJ,
	DEBUG_OP
};

typedef struct aml_node
{
	struct aml_node *children;
	struct aml_node *next;

	enum node_type type;

	const char *data;
	size_t data_length;
} aml_node_t;

typedef aml_node_t *(*parse_func_t)(const char **, size_t *);

aml_node_t *parse_node(enum node_type type, const char **src, size_t *len,
	size_t n, ...);
aml_node_t *parse_serie(const char **src, size_t *len, size_t n, ...);
aml_node_t *parse_list(enum node_type type, const char **src, size_t *len,
	parse_func_t f);
aml_node_t *parse_string(const char **src, size_t *len,
	size_t str_len, parse_func_t f);
aml_node_t *parse_either(const char **src, size_t *len, size_t n, ...);

aml_node_t *node_new(enum node_type type, const char *data, size_t length);
void node_add_child(aml_node_t *node, aml_node_t *child);
void node_free(aml_node_t *node);
void ast_free(aml_node_t *ast);

aml_node_t *byte_data(const char **src, size_t *len);
aml_node_t *word_data(const char **src, size_t *len);
aml_node_t *dword_data(const char **src, size_t *len);
aml_node_t *qword_data(const char **src, size_t *len);

aml_node_t *name_seg(const char **src, size_t *len);
aml_node_t *name_string(const char **src, size_t *len);

aml_node_t *access_type(const char **src, size_t *len);
aml_node_t *access_attrib(const char **src, size_t *len);
aml_node_t *extended_access_attrib(const char **src, size_t *len);
aml_node_t *access_length(const char **src, size_t *len);

aml_node_t *def_block_header(const char **src, size_t *len);

aml_node_t *pkg_length(const char **src, size_t *len);

aml_node_t *namespace_modifier_obj(const char **src, size_t *len);

aml_node_t *def_bank_field(const char **src, size_t *len);
aml_node_t *bank_value(const char **src, size_t *len);

aml_node_t *field_flags(const char **src, size_t *len);
aml_node_t *field_list(const char **src, size_t *len);

aml_node_t *named_obj(const char **src, size_t *len);

aml_node_t *data_ref_object(const char **src, size_t *len);

aml_node_t *def_break(const char **src, size_t *len);
aml_node_t *def_breakpoint(const char **src, size_t *len);
aml_node_t *def_continue(const char **src, size_t *len);
aml_node_t *def_else(const char **src, size_t *len);
aml_node_t *def_fatal(const char **src, size_t *len);
aml_node_t *def_ifelse(const char **src, size_t *len);
aml_node_t *predicate(const char **src, size_t *len);
aml_node_t *def_load(const char **src, size_t *len);
aml_node_t *def_noop(const char **src, size_t *len);
aml_node_t *def_notify(const char **src, size_t *len);
aml_node_t *def_release(const char **src, size_t *len);
aml_node_t *def_reset(const char **src, size_t *len);
aml_node_t *def_return(const char **src, size_t *len);
aml_node_t *def_signal(const char **src, size_t *len);
aml_node_t *def_sleep(const char **src, size_t *len);
aml_node_t *def_stall(const char **src, size_t *len);
aml_node_t *def_while(const char **src, size_t *len);

aml_node_t *type1_opcode(const char **src, size_t *len);
aml_node_t *type2_opcode(const char **src, size_t *len);

aml_node_t *term_list(const char **src, size_t *len);
aml_node_t *aml_parse(const char *src, const size_t len);

#endif
