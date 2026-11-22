//! Syntax highlighting (`JARAK-FR-090`, `JARAK-FR-091`, Step 98, ADR 0103): data-driven lexers —
//! a language is a [`Lang`] (comments, strings, keywords, operators, …) run by one of a few lexer
//! styles (code, markup, Markdown) — so user-defined languages and Notepad++'s UDL files work with
//! the same engine as the built-in ones.
//!
//! A line is lexed from the [`State`] its previous line ended in (inside a block comment, a
//! multi-line string, a tag, an embedded script, a fenced code block …), so highlighting needs only
//! the lines on screen once the states before them are known; [`Highlighter`] keeps those states
//! per line and drops the ones after an edit.

use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// What a span of text is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Keyword,
    Type,
    Constant,
    String,
    Number,
    Comment,
    Operator,
    Preprocessor,
    Tag,
    Attribute,
    Variable,
    Heading,
    Emphasis,
    Code,
    Link,
}

/// How a language is lexed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Style {
    /// Comments, strings, numbers, keywords, operators (with key/section rules for data formats).
    #[default]
    Code,
    /// HTML and XML: tags, attributes, comments, entities; HTML embeds scripts and styles.
    Markup,
    /// Markdown: headings, emphasis, code spans, links, fenced code in its language.
    Markdown,
}

/// A delimited span: a string or a block comment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Delim {
    pub open: String,
    pub close: String,
    /// The character that escapes the next one (`\`).
    #[serde(default)]
    pub escape: Option<char>,
    /// It may run over several lines.
    #[serde(default)]
    pub multiline: bool,
    /// It only opens at a line's start (Ruby's `=begin`).
    #[serde(default)]
    pub line_start: bool,
    /// It must close within this many characters, else it is not one (Rust's `'a` lifetimes).
    #[serde(default)]
    pub max_len: Option<usize>,
}

fn yes() -> bool {
    true
}

/// A language. The built-in ones are defined in code; user-defined languages are the same
/// structure in a JSON file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lang {
    pub name: String,
    #[serde(default)]
    pub extensions: Vec<String>,
    /// Interpreter names a `#!` line may name (`python3`, `bash`).
    #[serde(default)]
    pub shebang: Vec<String>,
    #[serde(default)]
    pub style: Style,
    #[serde(default = "yes")]
    pub case_sensitive: bool,
    #[serde(default)]
    pub line_comment: Vec<String>,
    /// Line comments that only start a line (after blanks and `@`), e.g. Batch's `rem`.
    #[serde(default)]
    pub line_start_comment: Vec<String>,
    #[serde(default)]
    pub block_comment: Vec<Delim>,
    #[serde(default)]
    pub strings: Vec<Delim>,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub types: Vec<String>,
    #[serde(default)]
    pub constants: Vec<String>,
    /// Operator characters.
    #[serde(default)]
    pub operators: String,
    /// Characters that start a variable (`$`, `@`); `%` means Batch's `%name%`.
    #[serde(default)]
    pub variable_prefix: String,
    /// A line starting with this is a preprocessor line (`#`).
    #[serde(default)]
    pub preprocessor: Option<String>,
    /// More characters allowed inside identifiers (`-` in CSS and PowerShell).
    #[serde(default)]
    pub ident_chars: String,
    /// A name or string followed by this is a key (`:` in JSON and YAML, `=` in INI and TOML).
    #[serde(default)]
    pub key_separator: Option<char>,
    /// A line starting with `[` is a section (INI, TOML).
    #[serde(default)]
    pub sections: bool,
    #[serde(default = "yes")]
    pub numbers: bool,
    /// Folding markers (open, close), for user-defined languages.
    #[serde(default)]
    pub fold: Vec<[String; 2]>,
}

impl Lang {
    fn new(name: &str) -> Lang {
        Lang {
            name: name.into(),
            extensions: Vec::new(),
            shebang: Vec::new(),
            style: Style::Code,
            case_sensitive: true,
            line_comment: Vec::new(),
            line_start_comment: Vec::new(),
            block_comment: Vec::new(),
            strings: Vec::new(),
            keywords: Vec::new(),
            types: Vec::new(),
            constants: Vec::new(),
            operators: "+-*/%=<>!&|^~?:;,.()[]{}".into(),
            variable_prefix: String::new(),
            preprocessor: None,
            ident_chars: String::new(),
            key_separator: None,
            sections: false,
            numbers: true,
            fold: Vec::new(),
        }
    }

    fn word_kind(&self, w: &str) -> Option<Kind> {
        let eq = |list: &Vec<String>| {
            if self.case_sensitive {
                list.iter().any(|k| k == w)
            } else {
                list.iter().any(|k| k.eq_ignore_ascii_case(w))
            }
        };
        if eq(&self.keywords) {
            Some(Kind::Keyword)
        } else if eq(&self.types) {
            Some(Kind::Type)
        } else if eq(&self.constants) {
            Some(Kind::Constant)
        } else {
            None
        }
    }
}

// ---- the built-in languages ---------------------------------------------------------------------------

fn words(s: &str) -> Vec<String> {
    s.split_whitespace().map(str::to_string).collect()
}

fn d(open: &str, close: &str, escape: Option<char>, multiline: bool) -> Delim {
    Delim {
        open: open.into(),
        close: close.into(),
        escape,
        multiline,
        line_start: false,
        max_len: None,
    }
}

fn c_family(name: &str, exts: &str, keywords: &str, types: &str, constants: &str) -> Lang {
    let mut l = Lang::new(name);
    l.extensions = words(exts);
    l.line_comment = words("//");
    l.block_comment = vec![d("/*", "*/", None, true)];
    l.strings = vec![
        d("\"", "\"", Some('\\'), false),
        d("'", "'", Some('\\'), false),
    ];
    l.keywords = words(keywords);
    l.types = words(types);
    l.constants = words(constants);
    l
}

fn build() -> Vec<Lang> {
    let mut v = Vec::new();
    let mut c = c_family(
        "C",
        "c h",
        "auto break case const continue default do else enum extern for goto if inline register restrict return sizeof static struct switch typedef union volatile while",
        "char double float int long short signed unsigned void bool size_t int8_t int16_t int32_t int64_t uint8_t uint16_t uint32_t uint64_t FILE",
        "NULL true false",
    );
    c.preprocessor = Some("#".into());
    v.push(c);
    let mut cpp = c_family(
        "C++",
        "cpp cc cxx hpp hh hxx ino",
        "alignas alignof and asm auto break case catch class const constexpr const_cast continue decltype default delete do dynamic_cast else enum explicit export extern for friend goto if inline mutable namespace new noexcept not operator or private protected public register reinterpret_cast return sizeof static static_assert static_cast struct switch template this thread_local throw try typedef typeid typename union using virtual volatile while co_await co_return co_yield concept requires",
        "bool char char8_t char16_t char32_t double float int long short signed unsigned void wchar_t size_t string vector map",
        "true false nullptr NULL",
    );
    cpp.preprocessor = Some("#".into());
    v.push(cpp);
    let mut cs = c_family(
        "C#",
        "cs csx",
        "abstract as base break case catch checked class const continue default delegate do else enum event explicit extern finally fixed for foreach goto if implicit in interface internal is lock namespace new operator out override params private protected public readonly ref return sealed sizeof stackalloc static struct switch this throw try typeof unchecked unsafe using virtual volatile while async await var record init get set where yield",
        "bool byte char decimal double float int long object sbyte short string uint ulong ushort void dynamic",
        "true false null",
    );
    cs.preprocessor = Some("#".into());
    cs.strings.push(d("@\"", "\"", None, true));
    v.push(cs);
    let mut rust = c_family(
        "Rust",
        "rs",
        "as async await break const continue crate dyn else enum extern fn for if impl in let loop match mod move mut pub ref return self Self static struct super trait type unsafe use where while yield macro_rules",
        "bool char f32 f64 i8 i16 i32 i64 i128 isize str u8 u16 u32 u64 u128 usize String Vec Option Result Box",
        "true false None Some Ok Err",
    );
    rust.strings = vec![
        d("\"", "\"", Some('\\'), true),
        Delim {
            max_len: Some(4),
            ..d("'", "'", Some('\\'), false)
        },
    ];
    v.push(rust);
    let mut go = c_family(
        "Go",
        "go",
        "break case chan const continue default defer else fallthrough for func go goto if import interface map package range return select struct switch type var",
        "bool byte complex64 complex128 error float32 float64 int int8 int16 int32 int64 rune string uint uint8 uint16 uint32 uint64 uintptr any",
        "true false nil iota",
    );
    go.strings.push(d("`", "`", None, true));
    v.push(go);
    v.push(c_family(
        "Java",
        "java",
        "abstract assert break case catch class const continue default do else enum extends final finally for goto if implements import instanceof interface native new package private protected public return static strictfp super switch synchronized this throw throws transient try volatile while var record sealed permits yield",
        "boolean byte char double float int long short void String Object Integer",
        "true false null",
    ));
    let js_kw = "async await break case catch class const continue debugger default delete do else export extends finally for from function if import in instanceof let new of return static super switch this throw try typeof var void while with yield";
    let mut js = c_family(
        "JavaScript",
        "js mjs cjs jsx",
        js_kw,
        "Array Object String Number Boolean Promise Map Set",
        "true false null undefined NaN Infinity",
    );
    js.strings.push(d("`", "`", Some('\\'), true));
    js.shebang = words("node");
    v.push(js);
    let mut ts = c_family(
        "TypeScript",
        "ts tsx mts cts",
        &format!(
            "{js_kw} abstract as declare enum implements interface keyof namespace private protected public readonly type"
        ),
        "any boolean never number object string symbol unknown void Array Promise Record",
        "true false null undefined",
    );
    ts.strings.push(d("`", "`", Some('\\'), true));
    v.push(ts);
    let mut py = Lang::new("Python");
    py.extensions = words("py pyw pyi");
    py.shebang = words("python python3 python2");
    py.line_comment = words("#");
    py.strings = vec![
        d("\"\"\"", "\"\"\"", Some('\\'), true),
        d("'''", "'''", Some('\\'), true),
        d("\"", "\"", Some('\\'), false),
        d("'", "'", Some('\\'), false),
    ];
    py.keywords = words(
        "and as assert async await break class continue def del elif else except finally for from global if import in is lambda nonlocal not or pass raise return try while with yield match case",
    );
    py.types = words("int float str bytes list dict set tuple bool object type range");
    py.constants = words("True False None");
    py.preprocessor = Some("@".into());
    v.push(py);
    let mut ps = Lang::new("PowerShell");
    ps.extensions = words("ps1 psm1 psd1");
    ps.shebang = words("pwsh powershell");
    ps.case_sensitive = false;
    ps.line_comment = words("#");
    ps.block_comment = vec![d("<#", "#>", None, true)];
    ps.strings = vec![
        d("@\"", "\"@", None, true),
        d("@'", "'@", None, true),
        d("\"", "\"", Some('`'), false),
        d("'", "'", None, false),
    ];
    ps.keywords = words(
        "begin break catch class continue data do dynamicparam else elseif end enum exit filter finally for foreach from function if in param process return switch throw trap try until using while -eq -ne -gt -ge -lt -le -like -notlike -match -notmatch -contains -notcontains -and -or -not -is -isnot",
    );
    ps.types = words("[string] [int] [bool] [array] [hashtable]");
    ps.constants = words("$true $false $null");
    ps.variable_prefix = "$".into();
    ps.ident_chars = "-".into();
    v.push(ps);
    let mut bat = Lang::new("Batch");
    bat.extensions = words("bat cmd");
    bat.case_sensitive = false;
    bat.line_start_comment = words("rem :: @rem");
    bat.strings = vec![d("\"", "\"", None, false)];
    bat.keywords = words(
        "echo set if else for in do goto call exit not exist defined errorlevel setlocal endlocal shift cd pushd popd start pause cls copy del move mkdir rmdir type",
    );
    bat.variable_prefix = "%!".into();
    bat.operators = "=<>|&()@".into();
    v.push(bat);
    let mut sh = Lang::new("Shell");
    sh.extensions = words("sh bash zsh ksh");
    sh.shebang = words("sh bash zsh ksh dash");
    sh.line_comment = words("#");
    sh.strings = vec![d("\"", "\"", Some('\\'), true), d("'", "'", None, true)];
    sh.keywords = words(
        "if then else elif fi case esac for select while until do done in function time return exit export local readonly declare unset shift source alias",
    );
    sh.constants = words("true false");
    sh.variable_prefix = "$".into();
    v.push(sh);
    let mut html = Lang::new("HTML");
    html.extensions = words("html htm xhtml shtml");
    html.style = Style::Markup;
    html.case_sensitive = false;
    v.push(html);
    let mut css = Lang::new("CSS");
    css.extensions = words("css scss less");
    css.block_comment = vec![d("/*", "*/", None, true)];
    css.strings = vec![
        d("\"", "\"", Some('\\'), false),
        d("'", "'", Some('\\'), false),
    ];
    css.ident_chars = "-".into();
    css.keywords = words("@media @import @font-face @keyframes @supports @charset !important");
    css.constants = words("inherit initial unset none auto");
    css.key_separator = Some(':');
    css.operators = "{}();:,.>+~*=[]".into();
    v.push(css);
    let mut json = Lang::new("JSON");
    json.extensions = words("json jsonc json5 geojson");
    json.line_comment = words("//");
    json.block_comment = vec![d("/*", "*/", None, true)];
    json.strings = vec![d("\"", "\"", Some('\\'), false)];
    json.constants = words("true false null");
    json.key_separator = Some(':');
    json.operators = "{}[]:,".into();
    v.push(json);
    let mut yaml = Lang::new("YAML");
    yaml.extensions = words("yaml yml");
    yaml.line_comment = words("#");
    yaml.strings = vec![d("\"", "\"", Some('\\'), false), d("'", "'", None, false)];
    yaml.constants = words("true false null yes no on off ~");
    yaml.key_separator = Some(':');
    yaml.variable_prefix = "&*".into();
    yaml.operators = "-:[]{},|>".into();
    v.push(yaml);
    let mut toml = Lang::new("TOML");
    toml.extensions = words("toml");
    toml.line_comment = words("#");
    toml.strings = vec![
        d("\"\"\"", "\"\"\"", Some('\\'), true),
        d("'''", "'''", None, true),
        d("\"", "\"", Some('\\'), false),
        d("'", "'", None, false),
    ];
    toml.constants = words("true false");
    toml.key_separator = Some('=');
    toml.sections = true;
    toml.operators = "=[]{},.".into();
    v.push(toml);
    let mut xml = Lang::new("XML");
    xml.extensions =
        words("xml xsd xsl xslt svg plist csproj vcxproj props targets config resx xaml");
    xml.style = Style::Markup;
    v.push(xml);
    let mut sql = Lang::new("SQL");
    sql.extensions = words("sql");
    sql.case_sensitive = false;
    sql.line_comment = words("--");
    sql.block_comment = vec![d("/*", "*/", None, true)];
    sql.strings = vec![
        d("'", "'", None, true),
        d("\"", "\"", None, false),
        d("`", "`", None, false),
    ];
    sql.keywords = words(
        "select from where insert into values update set delete create table drop alter add column index view join inner left right outer full on as and or not null is in between like order by group having limit offset union all distinct primary key foreign references default unique check case when then else end begin commit rollback transaction exists with returning",
    );
    sql.types = words(
        "int integer bigint smallint real float double decimal numeric char varchar text date time timestamp boolean blob",
    );
    sql.constants = words("true false");
    v.push(sql);
    let mut md = Lang::new("Markdown");
    md.extensions = words("md markdown mdown mkd mkdn");
    md.style = Style::Markdown;
    v.push(md);
    let mut ini = Lang::new("INI");
    ini.extensions = words("ini cfg inf reg properties conf editorconfig gitconfig");
    ini.line_start_comment = words("; #");
    ini.key_separator = Some('=');
    ini.sections = true;
    ini.numbers = true;
    ini.operators = "=".into();
    ini.strings = vec![d("\"", "\"", None, false)];
    v.push(ini);
    let mut lua = Lang::new("Lua");
    lua.extensions = words("lua");
    lua.shebang = words("lua");
    lua.block_comment = vec![d("--[[", "]]", None, true)];
    lua.line_comment = words("--");
    lua.strings = vec![
        d("[[", "]]", None, true),
        d("\"", "\"", Some('\\'), false),
        d("'", "'", Some('\\'), false),
    ];
    lua.keywords = words(
        "and break do else elseif end for function goto if in local not or repeat return then until while",
    );
    lua.constants = words("true false nil");
    v.push(lua);
    let mut php = Lang::new("PHP");
    php.extensions = words("php phtml php3 php4 php5 phps");
    php.shebang = words("php");
    php.style = Style::Markup;
    php.case_sensitive = false;
    v.push(php);
    let mut ruby = Lang::new("Ruby");
    ruby.extensions = words("rb rbw rake gemspec");
    ruby.shebang = words("ruby");
    ruby.line_comment = words("#");
    ruby.block_comment = vec![Delim {
        line_start: true,
        ..d("=begin", "=end", None, true)
    }];
    ruby.strings = vec![
        d("\"", "\"", Some('\\'), true),
        d("'", "'", Some('\\'), true),
    ];
    ruby.keywords = words(
        "alias and begin break case class def defined? do else elsif end ensure for if in module next not or redo rescue retry return self super then undef unless until when while yield require attr_accessor",
    );
    ruby.constants = words("true false nil");
    ruby.variable_prefix = "$@".into();
    v.push(ruby);
    // Step 110 (LANG-02): further Notepad++ languages.
    let mut perl = Lang::new("Perl");
    perl.extensions = words("pl pm t pod");
    perl.shebang = words("perl");
    perl.line_comment = words("#");
    perl.strings = vec![
        d("\"", "\"", Some('\\'), true),
        d("'", "'", Some('\\'), true),
    ];
    perl.keywords = words(
        "my our local sub package use require no if elsif else unless while until for foreach do last next redo return and or not eq ne lt gt le ge cmp print BEGIN END",
    );
    perl.variable_prefix = "$@%".into();
    v.push(perl);
    let mut pascal = Lang::new("Pascal");
    pascal.extensions = words("pas pp dpr dpk lpr");
    pascal.case_sensitive = false;
    pascal.line_comment = words("//");
    pascal.block_comment = vec![d("{", "}", None, true), d("(*", "*)", None, true)];
    pascal.strings = vec![d("'", "'", None, false)];
    pascal.keywords = words(
        "program unit uses interface implementation begin end var const type procedure function if then else case of for to downto do while repeat until with record array set class object private public protected published property constructor destructor inherited try except finally raise and or not div mod in is as nil",
    );
    pascal.types =
        words("integer real boolean char string byte word longint cardinal double single");
    pascal.constants = words("true false");
    v.push(pascal);
    let mut haskell = Lang::new("Haskell");
    haskell.extensions = words("hs lhs");
    haskell.line_comment = words("--");
    haskell.block_comment = vec![d("{-", "-}", None, true)];
    haskell.strings = vec![d("\"", "\"", Some('\\'), false)];
    haskell.keywords = words(
        "module import qualified as hiding where let in if then else case of data type newtype class instance deriving do forall infix infixl infixr",
    );
    haskell.types = words("Int Integer Double Float Bool Char String Maybe Either IO");
    haskell.constants = words("True False Nothing Just Left Right");
    v.push(haskell);
    let mut lisp = Lang::new("Lisp");
    lisp.extensions = words("lisp lsp cl el");
    lisp.line_comment = words(";");
    lisp.block_comment = vec![d("#|", "|#", None, true)];
    lisp.strings = vec![d("\"", "\"", Some('\\'), true)];
    lisp.keywords = words(
        "defun defmacro defvar defparameter let let* lambda if when unless cond case progn setq setf loop do dolist dotimes return quote function and or not",
    );
    lisp.constants = words("t nil");
    lisp.ident_chars = "-*!?<>=/+".into();
    v.push(lisp);
    let mut scheme = Lang::new("Scheme");
    scheme.extensions = words("scm ss sld rkt");
    scheme.line_comment = words(";");
    scheme.block_comment = vec![d("#|", "|#", None, true)];
    scheme.strings = vec![d("\"", "\"", Some('\\'), true)];
    scheme.keywords = words(
        "define lambda let let* letrec if cond case and or when unless begin do set! quote quasiquote define-syntax syntax-rules import library",
    );
    scheme.constants = words("#t #f");
    scheme.ident_chars = "-*!?<>=/+".into();
    v.push(scheme);
    let mut fortran = Lang::new("Fortran");
    fortran.extensions = words("f90 f95 f03 f08 for f ftn");
    fortran.case_sensitive = false;
    fortran.line_comment = words("!");
    fortran.strings = vec![d("\"", "\"", None, false), d("'", "'", None, false)];
    fortran.keywords = words(
        "program end module use implicit none contains subroutine function call return if then else elseif endif do enddo while select case stop print write read allocate deallocate intent in out inout parameter",
    );
    fortran.types = words("integer real double precision complex logical character");
    fortran.constants = words(".true. .false.");
    v.push(fortran);
    let mut ada = Lang::new("Ada");
    ada.extensions = words("adb ads ada");
    ada.case_sensitive = false;
    ada.line_comment = words("--");
    ada.strings = vec![d("\"", "\"", None, false)];
    ada.keywords = words(
        "procedure function package body is begin end if then elsif else case when loop while for in out return declare type subtype record array of access new null with use private limited task protected entry accept select exception raise and or not mod rem",
    );
    ada.types = words("Integer Float Boolean Character String Natural Positive");
    ada.constants = words("True False");
    v.push(ada);
    let mut asm = Lang::new("Assembly");
    asm.extensions = words("asm s nasm");
    asm.case_sensitive = false;
    asm.line_comment = words(";");
    asm.strings = vec![d("\"", "\"", None, false), d("'", "'", None, false)];
    asm.keywords = words(
        "mov push pop add sub mul imul div idiv inc dec and or xor not shl shr cmp test jmp je jne jz jnz jg jl jge jle call ret lea nop int section global extern db dw dd dq resb resw",
    );
    asm.types = words(
        "eax ebx ecx edx esi edi esp ebp rax rbx rcx rdx rsi rdi rsp rbp al ah bl bh cl ch dl dh",
    );
    v.push(asm);
    v.push(c_family(
        "D",
        "d di",
        "module import public private protected package static final abstract override class struct union interface enum template mixin alias auto ref const immutable shared if else switch case default for foreach while do break continue return goto try catch finally throw new delete is in out scope assert unittest",
        "void bool byte ubyte short ushort int uint long ulong float double real char wchar dchar string",
        "true false null",
    ));
    let mut r = Lang::new("R");
    r.extensions = words("r rmd");
    r.shebang = words("Rscript");
    r.line_comment = words("#");
    r.strings = vec![
        d("\"", "\"", Some('\\'), true),
        d("'", "'", Some('\\'), true),
    ];
    r.keywords = words("function if else for while repeat break next return in library require");
    r.constants = words("TRUE FALSE NULL NA NaN Inf T F");
    v.push(r);
    v.push(c_family(
        "Swift",
        "swift",
        "import class struct enum protocol extension func var let if else guard switch case default for in while repeat return break continue throw throws rethrows try catch do defer init deinit self super static public private internal fileprivate open override mutating where as is",
        "Int Double Float Bool String Character Array Dictionary Set Optional Any AnyObject Void",
        "true false nil",
    ));
    let mut vhdl = Lang::new("VHDL");
    vhdl.extensions = words("vhd vhdl");
    vhdl.case_sensitive = false;
    vhdl.line_comment = words("--");
    vhdl.strings = vec![d("\"", "\"", None, false)];
    vhdl.keywords = words(
        "library use entity architecture is begin end port map signal variable constant process if then elsif else case when others for loop generate component of in out inout downto to wait report severity",
    );
    vhdl.types =
        words("std_logic std_logic_vector integer natural boolean bit bit_vector unsigned signed");
    v.push(vhdl);
    let mut verilog = c_family(
        "Verilog",
        "v vh sv svh",
        "module endmodule input output inout wire reg logic assign always always_ff always_comb initial begin end if else case endcase for while parameter localparam function endfunction task endtask posedge negedge generate endgenerate",
        "integer real time",
        "",
    );
    verilog.preprocessor = Some("`".into());
    v.push(verilog);
    let mut cobol = Lang::new("COBOL");
    cobol.extensions = words("cob cbl cpy");
    cobol.case_sensitive = false;
    cobol.line_comment = words("*>");
    cobol.strings = vec![d("\"", "\"", None, false), d("'", "'", None, false)];
    cobol.keywords = words(
        "identification division program-id environment configuration data working-storage local-storage linkage file section procedure perform until varying from by times end-perform display accept move to if then else end-if evaluate when other end-evaluate compute add subtract multiply divide giving stop run goback call using pic picture value occurs copy",
    );
    cobol.constants = words("zero zeros zeroes space spaces high-value low-value");
    cobol.ident_chars = "-".into();
    v.push(cobol);
    let mut autoit = Lang::new("AutoIt");
    autoit.extensions = words("au3");
    autoit.case_sensitive = false;
    autoit.line_comment = words(";");
    autoit.strings = vec![d("\"", "\"", None, false), d("'", "'", None, false)];
    autoit.keywords = words(
        "func endfunc if then else elseif endif while wend for to step next do until select case endselect switch endswitch return local global dim const exit continueloop and or not",
    );
    autoit.variable_prefix = "$".into();
    autoit.preprocessor = Some("#".into());
    v.push(autoit);
    // The code inside PHP's `<?php … ?>` (not a language of its own in the menu).
    let mut phpc = c_family(
        "PHP code",
        "",
        "abstract and array as break callable case catch class clone const continue declare default do echo else elseif empty enddeclare endfor endforeach endif endswitch endwhile extends final finally fn for foreach function global goto if implements include include_once instanceof insteadof interface isset list match namespace new or print private protected public readonly require require_once return static switch throw trait try unset use var while xor yield",
        "int float string bool array object mixed void",
        "true false null TRUE FALSE NULL",
    );
    phpc.case_sensitive = false;
    phpc.line_comment = words("// #");
    phpc.variable_prefix = "$".into();
    v.push(phpc);
    v
}

/// The built-in languages (and the PHP code language used inside PHP files).
pub fn builtins() -> &'static [Lang] {
    static B: OnceLock<Vec<Lang>> = OnceLock::new();
    B.get_or_init(build)
}

/// The built-in language called `name` (any case).
pub fn builtin(name: &str) -> Option<&'static Lang> {
    builtins()
        .iter()
        .find(|l| l.name.eq_ignore_ascii_case(name))
}

/// The language of a file name (by extension) among `langs`, then the built-ins.
pub fn detect<'a>(file_name: &str, first_line: &str, user: &'a [Lang]) -> Option<&'a Lang>
where
    'static: 'a,
{
    let lower = file_name.to_lowercase();
    let ext = lower.rsplit_once('.').map(|(_, e)| e);
    let by_ext = |list: &'a [Lang]| {
        list.iter().find(|l| {
            ext.is_some_and(|e| l.extensions.iter().any(|x| x.eq_ignore_ascii_case(e)))
                || l.extensions.iter().any(|x| x.eq_ignore_ascii_case(&lower))
        })
    };
    if let Some(l) = by_ext(user).or_else(|| by_ext(builtins())) {
        return Some(l);
    }
    // `#!/usr/bin/env python3`, `#!/bin/bash`.
    let first = first_line.trim();
    if let Some(rest) = first.strip_prefix("#!") {
        let mut parts = rest.split_whitespace();
        let mut prog = parts.next().unwrap_or("").rsplit('/').next().unwrap_or("");
        if prog == "env" {
            prog = parts.next().unwrap_or("");
        }
        let prog = prog.to_lowercase();
        return user.iter().chain(builtins()).find(|l| {
            l.shebang.iter().any(|s| {
                *s == prog
                    || prog.starts_with(s.as_str())
                        && prog[s.len()..]
                            .chars()
                            .all(|c| c.is_ascii_digit() || c == '.')
            })
        });
    }
    if first.starts_with("<?xml") {
        return builtin("XML");
    }
    None
}

// ---- states and tokens -----------------------------------------------------------------------------------

/// Where a line starts.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum State {
    #[default]
    Normal,
    /// Inside the block comment number `n` of the language.
    Comment(u8),
    /// Inside the multi-line string number `n`.
    Str(u8),
    /// Markup: inside a tag (its name, lower case).
    Tag(String),
    /// Markup: inside `<!-- … -->` or `<![CDATA[ … ]]>`.
    MarkupComment,
    Cdata,
    /// Markup: inside an embedded language (`script`, `style`, PHP) with its own state; `end` is
    /// what ends it (`</script`, `?>`).
    Embedded(Box<(&'static Lang, State, &'static str)>),
    /// Markdown: inside a fenced code block (its fence, the language if known, its state).
    Fence(String, Option<&'static Lang>, Box<State>),
}

/// A highlighted span of a line, in characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token {
    pub start: usize,
    pub end: usize,
    pub kind: Kind,
}

struct Out {
    /// Byte spans.
    spans: Vec<(usize, usize, Kind)>,
}

impl Out {
    fn push(&mut self, a: usize, b: usize, k: Kind) {
        if b > a {
            self.spans.push((a, b, k));
        }
    }
}

fn starts_with_at(s: &str, at: usize, pat: &str, case_sensitive: bool) -> bool {
    let Some(rest) = s.get(at..) else {
        return false;
    };
    if case_sensitive {
        rest.starts_with(pat)
    } else {
        rest.get(..pat.len())
            .is_some_and(|x| x.eq_ignore_ascii_case(pat))
    }
}

fn is_ident(c: char, extra: &str) -> bool {
    c.is_alphanumeric() || c == '_' || extra.contains(c)
}

/// Lexes `line` of a code-style language from `st`; spans go to `out` (bytes, shifted by `base`).
fn lex_code(lang: &Lang, line: &str, base: usize, mut st: State, out: &mut Out) -> State {
    let b = line.as_bytes();
    let mut i = 0;
    // Continue a block comment or a multi-line string.
    match st {
        State::Comment(k) => {
            let dl = &lang.block_comment[k as usize];
            match find_close(line, 0, dl, lang.case_sensitive) {
                Some(e) => {
                    out.push(base, base + e, Kind::Comment);
                    i = e;
                    st = State::Normal;
                }
                None => {
                    out.push(base, base + line.len(), Kind::Comment);
                    return st;
                }
            }
        }
        State::Str(k) => {
            let dl = &lang.strings[k as usize];
            match find_close(line, 0, dl, lang.case_sensitive) {
                Some(e) => {
                    out.push(base, base + e, Kind::String);
                    i = e;
                    st = State::Normal;
                }
                None => {
                    out.push(base, base + line.len(), Kind::String);
                    return st;
                }
            }
        }
        _ => {}
    }
    let first_non_blank = line.len() - line.trim_start().len();
    // A preprocessor line or a line-start comment.
    if i == 0 {
        let lead = line.trim_start().trim_start_matches('@');
        let at = line.len() - lead.len();
        if lang.line_start_comment.iter().any(|c| {
            starts_with_at(line, at, c, lang.case_sensitive)
                && (c.len() < 3 || !lead[c.len()..].starts_with(|ch: char| ch.is_alphanumeric()))
        }) {
            out.push(base + first_non_blank, base + line.len(), Kind::Comment);
            return State::Normal;
        }
        if let Some(p) = &lang.preprocessor
            && line[first_non_blank..].starts_with(p.as_str())
            && line[first_non_blank + p.len()..].starts_with(|c: char| c.is_alphabetic())
        {
            out.push(
                base + first_non_blank,
                base + line.len(),
                Kind::Preprocessor,
            );
            return State::Normal;
        }
        if lang.sections && line[first_non_blank..].starts_with('[') {
            let end = line.rfind(']').map_or(line.len(), |e| e + 1);
            out.push(base + first_non_blank, base + end, Kind::Keyword);
            i = end;
        }
    }
    let ops = &lang.operators;
    let mut first_word = true;
    while i < b.len() {
        let c = line[i..].chars().next().unwrap_or(' ');
        // Block comments (before line comments: Lua's `--[[`).
        if let Some((k, dl)) = lang.block_comment.iter().enumerate().find(|(_, dl)| {
            (!dl.line_start || i == 0) && starts_with_at(line, i, &dl.open, lang.case_sensitive)
        }) {
            let from = i + dl.open.len();
            match find_close(line, from, dl, lang.case_sensitive) {
                Some(e) => {
                    out.push(base + i, base + e, Kind::Comment);
                    i = e;
                    continue;
                }
                None => {
                    out.push(base + i, base + line.len(), Kind::Comment);
                    return State::Comment(k as u8);
                }
            }
        }
        if lang
            .line_comment
            .iter()
            .any(|lc| starts_with_at(line, i, lc, lang.case_sensitive))
        {
            // `#` inside a word or `$#` in shells is not a comment.
            let prev = line[..i].chars().next_back();
            if !(c == '#' && prev.is_some_and(|p| is_ident(p, "$") || p == '{')) {
                out.push(base + i, base + line.len(), Kind::Comment);
                return State::Normal;
            }
        }
        if let Some((k, dl)) = lang
            .strings
            .iter()
            .enumerate()
            .find(|(_, dl)| starts_with_at(line, i, &dl.open, true))
        {
            let from = i + dl.open.len();
            match find_close(line, from, dl, true) {
                Some(e)
                    if dl
                        .max_len
                        .is_none_or(|m| line[from..e].chars().count() <= m + dl.close.len()) =>
                {
                    let kind = key_kind(lang, line, e).unwrap_or(Kind::String);
                    out.push(base + i, base + e, kind);
                    i = e;
                    first_word = false;
                    continue;
                }
                None if dl.multiline => {
                    out.push(base + i, base + line.len(), Kind::String);
                    return State::Str(k as u8);
                }
                None if dl.max_len.is_none() => {
                    out.push(base + i, base + line.len(), Kind::String);
                    return State::Normal;
                }
                _ => {}
            }
        }
        // Variables: `$name`, `${…}`, `%name%`, `!name!`, `@name`.
        if lang.variable_prefix.contains(c) && !c.is_alphanumeric() {
            let rest = &line[i + c.len_utf8()..];
            let len = if (c == '%' || c == '!') && lang.name == "Batch" {
                if rest.starts_with(|x: char| x.is_ascii_digit() || x == '~' || x == '*') {
                    1 + rest
                        .chars()
                        .take_while(|x| x.is_ascii_alphanumeric() || *x == '~')
                        .map(char::len_utf8)
                        .sum::<usize>()
                } else {
                    rest.find(c).map_or(0, |e| e + 2)
                }
            } else if rest.starts_with('{') {
                rest.find('}').map_or(0, |e| e + 2)
            } else {
                let n: usize = rest
                    .chars()
                    .take_while(|x| is_ident(*x, ":"))
                    .map(char::len_utf8)
                    .sum();
                if n > 0 { n + c.len_utf8() } else { 0 }
            };
            if len > 0 {
                let word = &line[i..i + len];
                let kind = if lang.word_kind(word) == Some(Kind::Constant) {
                    Kind::Constant
                } else {
                    Kind::Variable
                };
                out.push(base + i, base + i + len, kind);
                i += len;
                first_word = false;
                continue;
            }
        }
        if lang.numbers
            && (c.is_ascii_digit()
                || (c == '.' && line[i + 1..].starts_with(|x: char| x.is_ascii_digit())))
            && !line[..i]
                .chars()
                .next_back()
                .is_some_and(|p| is_ident(p, &lang.ident_chars))
        {
            let mut e = i;
            let mut prev = ' ';
            for ch in line[i..].chars() {
                let ok = ch.is_ascii_alphanumeric()
                    || ch == '_'
                    || ch == '.'
                    || ((ch == '-' || ch == '+') && (prev == 'e' || prev == 'E'))
                    || ((ch == ':' || ch == '-')
                        && lang.key_separator == Some('=')
                        && prev.is_ascii_digit());
                if !ok {
                    break;
                }
                prev = ch;
                e += ch.len_utf8();
            }
            out.push(base + i, base + e, Kind::Number);
            i = e;
            first_word = false;
            continue;
        }
        // A CSS colour `#a0b0c0`.
        if c == '#' && lang.name == "CSS" {
            let n: usize = line[i + 1..]
                .chars()
                .take_while(char::is_ascii_hexdigit)
                .count();
            if n == 3 || n == 6 || n == 8 {
                out.push(base + i, base + i + 1 + n, Kind::Number);
                i += 1 + n;
                continue;
            }
        }
        let starts_word = is_ident(c, "")
            || (lang.ident_chars.contains(c)
                && line[i + c.len_utf8()..].starts_with(|x: char| x.is_alphabetic()))
            || (c == '@' && lang.keywords.iter().any(|k| k.starts_with('@')))
            || c == '[' && lang.types.iter().any(|t| t.starts_with('['));
        if starts_word {
            let mut e = i + c.len_utf8();
            if c == '[' {
                e = line[i..].find(']').map_or(line.len(), |x| i + x + 1);
            } else {
                e += line[e..]
                    .chars()
                    .take_while(|x| {
                        is_ident(*x, &lang.ident_chars) || (*x == '?' && lang.name == "Ruby")
                    })
                    .map(char::len_utf8)
                    .sum::<usize>();
            }
            let w = &line[i..e];
            let kind = lang.word_kind(w).or_else(|| {
                key_kind(lang, line, e)
                    .filter(|_| first_word || lang.name == "CSS" || lang.name == "JSON")
            });
            if let Some(k) = kind {
                out.push(base + i, base + e, k);
            }
            i = e;
            first_word = false;
            continue;
        }
        if ops.contains(c) {
            out.push(base + i, base + i + c.len_utf8(), Kind::Operator);
        } else if c == '!' && lang.keywords.iter().any(|k| k.starts_with('!')) {
            let e = i + line[i..]
                .chars()
                .take_while(|x| *x == '!' || x.is_alphabetic())
                .map(char::len_utf8)
                .sum::<usize>();
            if lang.word_kind(&line[i..e]).is_some() {
                out.push(base + i, base + e, Kind::Keyword);
                i = e;
                continue;
            }
        }
        if !c.is_whitespace() {
            first_word = false;
        }
        i += c.len_utf8();
    }
    st
}

/// A word or string that ends at `e` and is followed by the key separator is a key.
fn key_kind(lang: &Lang, line: &str, e: usize) -> Option<Kind> {
    let sep = lang.key_separator?;
    let rest = line[e..].trim_start_matches([' ', '\t']);
    let after = rest.strip_prefix(sep)?;
    // YAML and CSS want a blank or the end after `:`; `a:hover` is a selector.
    if sep == ':' && lang.name != "JSON" && !(after.is_empty() || after.starts_with([' ', '\t'])) {
        return None;
    }
    Some(Kind::Attribute)
}

/// Where `dl` closes in `line` from byte `from` (the byte after the close), honouring escapes.
fn find_close(line: &str, from: usize, dl: &Delim, case_sensitive: bool) -> Option<usize> {
    let mut i = from;
    while i < line.len() {
        if dl.line_start && i == 0 && starts_with_at(line, 0, &dl.close, case_sensitive) {
            return Some(line.len());
        }
        if !dl.line_start && starts_with_at(line, i, &dl.close, case_sensitive) {
            return Some(i + dl.close.len());
        }
        let c = line[i..].chars().next()?;
        if Some(c) == dl.escape {
            i += c.len_utf8();
            if let Some(n) = line[i..].chars().next() {
                i += n.len_utf8();
            }
            continue;
        }
        if dl.line_start {
            return None;
        }
        i += c.len_utf8();
    }
    // An empty line can still be the closing line of a line-start delimiter.
    (dl.line_start && line.starts_with(&dl.close)).then_some(line.len())
}

/// Markup (HTML, XML, PHP files): tags, attributes, values, comments, entities, embedded code.
fn lex_markup(lang: &Lang, line: &str, base: usize, mut st: State, out: &mut Out) -> State {
    let php = lang.name == "PHP";
    let html = lang.name == "HTML" || php;
    let mut i = 0;
    while i <= line.len() {
        match st {
            State::MarkupComment | State::Cdata => {
                let close = if st == State::Cdata { "]]>" } else { "-->" };
                let kind = if st == State::Cdata {
                    Kind::String
                } else {
                    Kind::Comment
                };
                match line[i..].find(close) {
                    Some(e) => {
                        out.push(base + i, base + i + e + close.len(), kind);
                        i += e + close.len();
                        st = State::Normal;
                    }
                    None => {
                        out.push(base + i, base + line.len(), kind);
                        return st;
                    }
                }
            }
            State::Embedded(ref emb) => {
                let (inner_lang, inner_st, end) = (emb.0, emb.1.clone(), emb.2);
                let lower = line[i..].to_lowercase();
                let stop = lower.find(end).map(|e| i + e);
                let seg_end = stop.unwrap_or(line.len());
                let next = if inner_lang.style == Style::Markup {
                    lex_markup(inner_lang, &line[i..seg_end], base + i, inner_st, out)
                } else {
                    lex_code(inner_lang, &line[i..seg_end], base + i, inner_st, out)
                };
                match stop {
                    Some(e) => {
                        if end == "?>" {
                            out.push(base + e, base + e + 2, Kind::Preprocessor);
                            i = e + 2;
                        } else {
                            i = e;
                        }
                        st = State::Normal;
                    }
                    None => {
                        return State::Embedded(Box::new((inner_lang, next, end)));
                    }
                }
            }
            State::Tag(ref name) => {
                let name = name.clone();
                let rest = &line[i..];
                let skip = rest.len() - rest.trim_start().len();
                i += skip;
                if i >= line.len() {
                    return State::Tag(name);
                }
                let r = &line[i..];
                if r.starts_with("/>") || r.starts_with("?>") {
                    out.push(
                        base + i,
                        base + i + 2,
                        if r.starts_with('?') {
                            Kind::Preprocessor
                        } else {
                            Kind::Tag
                        },
                    );
                    i += 2;
                    st = State::Normal;
                } else if r.starts_with('>') {
                    out.push(base + i, base + i + 1, Kind::Tag);
                    i += 1;
                    st = if html && name == "script" {
                        State::Embedded(Box::new((
                            builtin("JavaScript").expect("built in"),
                            State::Normal,
                            "</script",
                        )))
                    } else if html && name == "style" {
                        State::Embedded(Box::new((
                            builtin("CSS").expect("built in"),
                            State::Normal,
                            "</style",
                        )))
                    } else {
                        State::Normal
                    };
                } else if r.starts_with('"') || r.starts_with('\'') {
                    let q = &r[..1];
                    let e = r[1..].find(q).map_or(r.len(), |e| e + 2);
                    out.push(base + i, base + i + e, Kind::String);
                    i += e;
                } else if r.starts_with('=') {
                    out.push(base + i, base + i + 1, Kind::Operator);
                    i += 1;
                } else {
                    let n: usize = r
                        .chars()
                        .take_while(|c| !c.is_whitespace() && !"=>/\"'".contains(*c))
                        .map(char::len_utf8)
                        .sum::<usize>()
                        .max(1);
                    out.push(base + i, base + i + n, Kind::Attribute);
                    i += n;
                }
                if i >= line.len() && matches!(st, State::Tag(_)) {
                    return st;
                }
            }
            _ => {
                let Some(off) = line[i..].find(['<', '&']) else {
                    return State::Normal;
                };
                i += off;
                let r = &line[i..];
                if r.starts_with("<!--") {
                    st = State::MarkupComment;
                    out.push(base + i, base + i + 4, Kind::Comment);
                    i += 4;
                } else if r.starts_with("<![CDATA[") {
                    st = State::Cdata;
                    out.push(base + i, base + i + 9, Kind::String);
                    i += 9;
                } else if php
                    && (r.starts_with("<?php") || r.starts_with("<?=") || r.starts_with("<? "))
                {
                    let n = if r.starts_with("<?php") { 5 } else { 3 };
                    out.push(base + i, base + i + n, Kind::Preprocessor);
                    i += n;
                    st = State::Embedded(Box::new((
                        builtin("PHP code").expect("built in"),
                        State::Normal,
                        "?>",
                    )));
                } else if r.starts_with("<?") || r.starts_with("<!") {
                    let n: usize = 2 + r[2..]
                        .chars()
                        .take_while(|c| c.is_alphanumeric())
                        .map(char::len_utf8)
                        .sum::<usize>();
                    out.push(base + i, base + i + n, Kind::Preprocessor);
                    i += n;
                    st = State::Tag("?".into());
                } else if r.starts_with('<')
                    && r[1..].starts_with(|c: char| c.is_alphabetic() || c == '/')
                {
                    let n: usize = 1 + r[1..]
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || "/:-_.".contains(*c))
                        .map(char::len_utf8)
                        .sum::<usize>();
                    let name = r[1..n].trim_start_matches('/').to_lowercase();
                    let closing = r[1..].starts_with('/');
                    out.push(base + i, base + i + n, Kind::Tag);
                    i += n;
                    st = State::Tag(if closing { String::new() } else { name });
                } else if r.starts_with('&') {
                    let n = r.find(';').filter(|e| {
                        *e < 12 && r[1..*e].chars().all(|c| c.is_alphanumeric() || c == '#')
                    });
                    match n {
                        Some(e) => {
                            out.push(base + i, base + i + e + 1, Kind::Constant);
                            i += e + 1;
                        }
                        None => i += 1,
                    }
                } else {
                    i += 1;
                }
            }
        }
    }
    st
}

/// Markdown: headings, quotes, list markers, emphasis, code spans, links; fenced code in its
/// language (the info string's first word: `rust`, `py`, `js` …).
fn lex_markdown(line: &str, base: usize, st: State, out: &mut Out) -> State {
    let t = line.trim_start();
    let lead = line.len() - t.len();
    if let State::Fence(fence, inner, inner_st) = st {
        if t.starts_with(fence.as_str())
            && t.trim_end()
                .chars()
                .all(|c| c == fence.chars().next().unwrap_or('`'))
        {
            out.push(base, base + line.len(), Kind::Code);
            return State::Normal;
        }
        return match inner {
            Some(l) => {
                let next = if l.style == Style::Markup {
                    lex_markup(l, line, base, *inner_st, out)
                } else {
                    lex_code(l, line, base, *inner_st, out)
                };
                State::Fence(fence, inner, Box::new(next))
            }
            None => {
                out.push(base, base + line.len(), Kind::Code);
                State::Fence(fence, None, Box::new(State::Normal))
            }
        };
    }
    for f in ["```", "~~~"] {
        if t.starts_with(f) {
            let n = t
                .chars()
                .take_while(|c| *c == f.chars().next().unwrap_or('`'))
                .count();
            let info = t[n..]
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_lowercase();
            out.push(base, base + line.len(), Kind::Code);
            return State::Fence(
                t[..n].to_string(),
                fence_lang(&info),
                Box::new(State::Normal),
            );
        }
    }
    if t.starts_with('#') && t.trim_start_matches('#').starts_with([' ', '\t'])
        || (t.starts_with('#') && t.trim_start_matches('#').is_empty())
    {
        out.push(base + lead, base + line.len(), Kind::Heading);
        return State::Normal;
    }
    let mut i = lead;
    if t.starts_with('>') {
        out.push(base + i, base + i + 1, Kind::Comment);
        i += 1;
    } else if (t.starts_with("- ") || t.starts_with("* ") || t.starts_with("+ "))
        || t.chars().take_while(char::is_ascii_digit).count() > 0
            && t.trim_start_matches(|c: char| c.is_ascii_digit())
                .starts_with(". ")
    {
        let n = t.find(' ').unwrap_or(0);
        out.push(base + i, base + i + n, Kind::Operator);
        i += n;
    }
    if t.chars()
        .all(|c| c == '-' || c == '*' || c == '_' || c == ' ')
        && t.chars().filter(|c| !c.is_whitespace()).count() >= 3
    {
        out.push(base + lead, base + line.len(), Kind::Operator);
        return State::Normal;
    }
    let b = line.as_bytes();
    while i < b.len() {
        let r = &line[i..];
        if r.starts_with('`') {
            let n = r.chars().take_while(|c| *c == '`').count();
            let close = &r[..n];
            if let Some(e) = r[n..].find(close) {
                out.push(base + i, base + i + n + e + n, Kind::Code);
                i += n + e + n;
                continue;
            }
        } else if r.starts_with("**") || r.starts_with("__") {
            if let Some(e) = r[2..].find(&r[..2]) {
                out.push(base + i, base + i + e + 4, Kind::Emphasis);
                i += e + 4;
                continue;
            }
        } else if (r.starts_with('*') || r.starts_with('_'))
            && r[1..].starts_with(|c: char| !c.is_whitespace())
            && let Some(e) = r[1..].find(&r[..1])
            && e > 0
        {
            out.push(base + i, base + i + e + 2, Kind::Emphasis);
            i += e + 2;
            continue;
        } else if r.starts_with("![") || r.starts_with('[') {
            let s = usize::from(r.starts_with('!'));
            if let Some(cb) = r.find("](")
                && let Some(cp) = r[cb..].find(')')
            {
                out.push(base + i, base + i + cb + 1, Kind::Link);
                out.push(base + i + cb + 1, base + i + cb + cp + 1, Kind::String);
                i += cb + cp + 1;
                continue;
            }
            let _ = s;
        } else if (r.starts_with("<http") || r.starts_with("<mailto:"))
            && let Some(e) = r.find('>')
        {
            out.push(base + i, base + i + e + 1, Kind::Link);
            i += e + 1;
            continue;
        }
        i += r.chars().next().map_or(1, char::len_utf8);
    }
    State::Normal
}

/// The language of a fence's info string.
/// The language a fenced code block's info string names (`rs`, `py`, `Rust`, …).
pub fn fence_lang(info: &str) -> Option<&'static Lang> {
    let alias = match info {
        "rs" => "rust",
        "py" | "python3" => "python",
        "js" | "node" => "javascript",
        "ts" => "typescript",
        "sh" | "bash" | "zsh" | "console" => "shell",
        "ps" | "ps1" | "pwsh" => "powershell",
        "bat" | "cmd" => "batch",
        "cs" | "csharp" => "c#",
        "cpp" | "cxx" | "hpp" => "c++",
        "yml" => "yaml",
        "rb" => "ruby",
        "md" => "markdown",
        "htm" => "html",
        "golang" => "go",
        x => x,
    };
    builtin(alias).filter(|l| l.style != Style::Markdown && l.name != "PHP code")
}

/// Lexes one line from `st`: its tokens (in characters) and the state the next line starts in.
pub fn lex_line(lang: &Lang, line: &str, st: State) -> (Vec<Token>, State) {
    let mut out = Out { spans: Vec::new() };
    let next = match lang.style {
        Style::Code => lex_code(lang, line, 0, st, &mut out),
        Style::Markup => lex_markup(lang, line, 0, st, &mut out),
        Style::Markdown => lex_markdown(line, 0, st, &mut out),
    };
    // Bytes to characters (spans come in order).
    let mut toks = Vec::with_capacity(out.spans.len());
    let mut ci = 0;
    let mut bi = 0;
    let mut char_at = |b: usize| {
        while bi < b && bi < line.len() {
            bi += line[bi..].chars().next().map_or(1, char::len_utf8);
            ci += 1;
        }
        ci
    };
    let mut spans = out.spans;
    spans.sort_by_key(|s| s.0);
    for (a, b, k) in spans {
        let (a, b) = (a.min(line.len()), b.min(line.len()));
        let ca = char_at(a);
        let cb = char_at(b);
        if cb > ca {
            toks.push(Token {
                start: ca,
                end: cb,
                kind: k,
            });
        }
    }
    (toks, next)
}

/// The line-start states of a document, kept as far as they were needed and dropped after an
/// edit's line.
#[derive(Debug, Clone, Default)]
pub struct Highlighter {
    /// `states[i]` is the state line `i` starts in.
    states: Vec<State>,
}

impl Highlighter {
    /// Forgets the states from line `line` on (an edit there).
    pub fn invalidate_from(&mut self, line: usize) {
        self.states.truncate(line + 1);
    }

    /// How many line-start states are known.
    pub fn known(&self) -> usize {
        self.states.len()
    }

    /// The tokens of line `line`; `text_of(i)` gives line `i` without its line break. Lexes the
    /// lines before it once, as far as no state is known yet.
    pub fn line(
        &mut self,
        lang: &Lang,
        line: usize,
        text_of: &dyn Fn(usize) -> String,
    ) -> Vec<Token> {
        if self.states.is_empty() {
            self.states.push(State::Normal);
        }
        while self.states.len() <= line {
            let i = self.states.len() - 1;
            let (_, next) = lex_line(lang, &text_of(i), self.states[i].clone());
            self.states.push(next);
        }
        let (toks, next) = lex_line(lang, &text_of(line), self.states[line].clone());
        if self.states.len() == line + 1 {
            self.states.push(next);
        } else if self.states[line + 1] != next {
            // The state changed: the lines after need lexing again.
            self.states.truncate(line + 1);
            self.states.push(next);
        }
        toks
    }
}

/// The fold ranges (first line, last line) of a language with folding markers: an open marker
/// (a word or symbol, outside comments and strings) starts a range the matching close ends.
pub fn fold_ranges(lang: &Lang, lines: &[String]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    if lang.fold.is_empty() {
        return out;
    }
    let mut stack: Vec<(usize, usize)> = Vec::new();
    let mut st = State::Normal;
    for (n, line) in lines.iter().enumerate() {
        let (toks, next) = lex_line(lang, line, st);
        st = next;
        let chars: Vec<char> = line.chars().collect();
        let inside = |c: usize| {
            toks.iter().any(|t| {
                (t.kind == Kind::Comment || t.kind == Kind::String) && t.start <= c && c < t.end
            })
        };
        let mut c = 0;
        while c < chars.len() {
            let rest: String = chars[c..].iter().collect();
            let mut matched = false;
            for (k, [open, close]) in lang.fold.iter().enumerate() {
                let word_ok = |m: &str| {
                    let wordy = m.chars().all(|x| x.is_alphanumeric() || x == '_');
                    !wordy
                        || (c == 0 || !chars[c - 1].is_alphanumeric())
                            && !chars
                                .get(c + m.chars().count())
                                .is_some_and(|x| x.is_alphanumeric())
                };
                let eq = |m: &str| {
                    if lang.case_sensitive {
                        rest.starts_with(m)
                    } else {
                        rest.to_lowercase().starts_with(&m.to_lowercase())
                    }
                };
                if !inside(c) && eq(open) && word_ok(open) {
                    stack.push((k, n));
                    c += open.chars().count();
                    matched = true;
                    break;
                }
                if !inside(c) && eq(close) && word_ok(close) {
                    if let Some(pos) = stack.iter().rposition(|(kk, _)| *kk == k) {
                        let (_, from) = stack.remove(pos);
                        if n > from {
                            out.push((from, n));
                        }
                    }
                    c += close.chars().count();
                    matched = true;
                    break;
                }
            }
            if !matched {
                c += 1;
            }
        }
    }
    out.sort_unstable();
    out
}

// ---- Notepad++ UDL import -----------------------------------------------------------------------------

pub(crate) fn xml_unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(p) = rest.find('&') {
        out.push_str(&rest[..p]);
        let r = &rest[p..];
        match r.find(';') {
            Some(e) if e < 10 => {
                let ent = &r[1..e];
                let ch = match ent {
                    "lt" => Some('<'),
                    "gt" => Some('>'),
                    "amp" => Some('&'),
                    "quot" => Some('"'),
                    "apos" => Some('\''),
                    _ if ent.starts_with("#x") => u32::from_str_radix(&ent[2..], 16)
                        .ok()
                        .and_then(char::from_u32),
                    _ if ent.starts_with('#') => ent[1..].parse().ok().and_then(char::from_u32),
                    _ => None,
                };
                match ch {
                    Some(c) => out.push(c),
                    None => out.push_str(&r[..=e]),
                }
                rest = &r[e + 1..];
            }
            _ => {
                out.push('&');
                rest = &r[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn attr(tag: &str, name: &str) -> Option<String> {
    let key = format!("{name}=");
    let p = tag.find(&key)? + key.len();
    let q = tag[p..].chars().next()?;
    let rest = &tag[p + 1..];
    let e = rest.find(q)?;
    Some(xml_unescape(&rest[..e]))
}

/// Notepad++ keyword-list values: `00// 01 02 03/* 04*/` → group number → its words.
fn grouped(s: &str) -> Vec<(u32, String)> {
    let mut out = Vec::new();
    for w in s.split_whitespace() {
        if w.len() >= 2 && w[..2].chars().all(|c| c.is_ascii_digit()) {
            let g: u32 = w[..2].parse().unwrap_or(99);
            if w.len() > 2 {
                out.push((g, w[2..].to_string()));
            }
        }
    }
    out
}

/// Converts a Notepad++ user-defined language (UDL 2.x XML) into a [`Lang`]. Taken: the name, the
/// extensions, case, line and block comments, the eight keyword lists (1 → keywords, 2 → types,
/// 3 → constants, 4–8 → keywords), the operators, up to eight delimiters with their escapes as
/// strings, and the code folding markers. Not taken (documented): styles and colours, prefix and
/// number definitions, comment folding, "line comment continue" and positions.
pub fn import_npp_udl(xml: &str) -> Result<Lang, String> {
    let ul = xml
        .find("<UserLang")
        .ok_or("not a Notepad++ user-defined language (no <UserLang>)")?;
    let tag_end = xml[ul..].find('>').ok_or("an unterminated <UserLang>")? + ul;
    let tag = &xml[ul..tag_end];
    let name = attr(tag, "name")
        .filter(|n| !n.trim().is_empty())
        .ok_or("the language has no name")?;
    let mut l = Lang::new(&name);
    l.operators.clear();
    l.extensions = attr(tag, "ext").map(|e| words(&e)).unwrap_or_default();
    if let Some(g) = xml.find("<Global") {
        let gt = &xml[g..g + xml[g..].find('>').unwrap_or(0)];
        l.case_sensitive = attr(gt, "caseIgnored").is_none_or(|v| v != "yes");
    }
    let mut at = 0;
    let mut fold_open = Vec::new();
    let mut fold_close = Vec::new();
    while let Some(p) = xml[at..].find("<Keywords ") {
        let s = at + p;
        let te = s + xml[s..].find('>').ok_or("an unterminated <Keywords>")?;
        let ktag = &xml[s..te];
        let kname = attr(ktag, "name").unwrap_or_default();
        let (text, next) = if ktag.ends_with('/') {
            (String::new(), te + 1)
        } else {
            let ce = te
                + xml[te..]
                    .find("</Keywords>")
                    .ok_or("an unterminated <Keywords>")?;
            (xml_unescape(&xml[te + 1..ce]), ce + 11)
        };
        at = next;
        match kname.as_str() {
            "Comments" => {
                let g = grouped(&text);
                l.line_comment = g
                    .iter()
                    .filter(|(n, _)| *n == 0)
                    .map(|(_, w)| w.clone())
                    .collect();
                let opens: Vec<&String> =
                    g.iter().filter(|(n, _)| *n == 3).map(|(_, w)| w).collect();
                let closes: Vec<&String> =
                    g.iter().filter(|(n, _)| *n == 4).map(|(_, w)| w).collect();
                l.block_comment = opens
                    .iter()
                    .zip(closes.iter())
                    .map(|(o, c)| d(o, c, None, true))
                    .collect();
            }
            "Operators1" | "Operators2" => {
                for w in text.split_whitespace() {
                    l.operators.push_str(w);
                }
            }
            "Folders in code1, open" | "Folders in code2, open" => fold_open.extend(words(&text)),
            "Folders in code1, close" | "Folders in code2, close" => {
                fold_close.extend(words(&text));
            }
            "Keywords1" | "Keywords4" | "Keywords5" | "Keywords6" | "Keywords7" | "Keywords8" => {
                l.keywords.extend(words(&text));
            }
            "Keywords2" => l.types.extend(words(&text)),
            "Keywords3" => l.constants.extend(words(&text)),
            "Delimiters" => {
                let g = grouped(&text);
                for k in 0..8u32 {
                    let get = |n: u32| g.iter().find(|(x, _)| *x == n).map(|(_, w)| w.clone());
                    if let (Some(o), Some(c)) = (get(k * 3), get(k * 3 + 2)) {
                        let esc = get(k * 3 + 1).and_then(|e| e.chars().next());
                        l.strings.push(d(&o, &c, esc, false));
                    }
                }
            }
            _ => {}
        }
    }
    l.fold = fold_open
        .into_iter()
        .zip(fold_close)
        .map(|(o, c)| [o, c])
        .collect();
    Ok(l)
}

/// A user-defined language from its JSON file.
pub fn parse_udl(json: &str) -> Result<Lang, String> {
    let l: Lang = serde_json::from_str(json).map_err(|e| e.to_string())?;
    if l.name.trim().is_empty() {
        return Err("the language has no name".into());
    }
    Ok(l)
}

/// The user-defined languages in `dir` (`*.json`; a broken file is skipped with its reason).
pub fn load_udls(dir: &std::path::Path) -> (Vec<Lang>, Vec<(std::path::PathBuf, String)>) {
    let mut ok = Vec::new();
    let mut bad = Vec::new();
    let Ok(rd) = std::fs::read_dir(dir) else {
        return (ok, bad);
    };
    let mut files: Vec<std::path::PathBuf> = rd
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("json"))
        })
        .collect();
    files.sort();
    for p in files {
        match std::fs::read_to_string(&p)
            .map_err(|e| e.to_string())
            .and_then(|s| parse_udl(&s))
        {
            Ok(l) => ok.push(l),
            Err(e) => bad.push((p, e)),
        }
    }
    (ok, bad)
}

#[cfg(test)]
mod tests;
