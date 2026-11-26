import importlib.util
from pathlib import Path
import sqlite3
import tempfile
import unittest

script=Path(__file__).resolve().parents[1]/"apps/desktop/src-tauri/resources/account_recovery.py"
spec=importlib.util.spec_from_file_location("account_recovery",script)
module=importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)

class RecoveryTests(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup);self.root=Path(self.temp.name);self.db=self.root/"webui.db"
        with sqlite3.connect(self.db) as c:
            c.executescript('CREATE TABLE user(id TEXT PRIMARY KEY,name TEXT,role TEXT); CREATE TABLE auth(id TEXT PRIMARY KEY,email TEXT,password TEXT,active INTEGER,session_stamp TEXT); CREATE TABLE knowledge(id TEXT,user_id TEXT); CREATE TABLE chat(id TEXT,user_id TEXT,content TEXT);')
            c.execute('INSERT INTO user VALUES(?,?,?)',('user-1','Owner','admin'))
            c.execute('INSERT INTO auth VALUES(?,?,?,?,?)',('user-1','owner@example.test','old-hash',1,'old-stamp'))
            c.execute('INSERT INTO knowledge VALUES(?,?)',('eight-textbooks','user-1'))
            c.execute('INSERT INTO chat VALUES(?,?,?)',('chat-1','user-1','keep conversation'))
        (self.root/'book.pdf').write_bytes(b'private fixture book')
    def test_reset_preserves_user_books_chats_and_backs_up_database(self):
        result=module.recover(self.db,{'action':'reset','email':'OWNER@example.test','password':'my new app password'},hasher=lambda _: 'new-hash')
        with sqlite3.connect(self.db) as c:
            self.assertEqual(c.execute('SELECT password FROM auth').fetchone()[0],'new-hash')
            self.assertNotEqual(c.execute('SELECT session_stamp FROM auth').fetchone()[0],'old-stamp')
            self.assertEqual(c.execute('SELECT * FROM knowledge').fetchone(),('eight-textbooks','user-1'))
            self.assertEqual(c.execute('SELECT content FROM chat').fetchone()[0],'keep conversation')
            self.assertEqual(c.execute('SELECT id FROM user').fetchone()[0],'user-1')
        with sqlite3.connect(self.root/'hermes-recovery-backups'/result['backup']) as c:self.assertEqual(c.execute('SELECT password FROM auth').fetchone()[0],'old-hash')
        self.assertEqual((self.root/'book.pdf').read_bytes(),b'private fixture book')
    def test_unknown_account_and_sql_injection_cannot_update_other_accounts(self):
        with self.assertRaises(ValueError):module.recover(self.db,{'action':'reset','email':"x' OR 1=1 --@example.test",'password':'my new app password'},hasher=lambda _: 'new-hash')
        with sqlite3.connect(self.db) as c:self.assertEqual(c.execute('SELECT password FROM auth').fetchone()[0],'old-hash')
    def test_lists_only_active_local_admins(self):
        self.assertEqual(module.recover(self.db,{'action':'list'})['accounts'],[{'id':'user-1','name':'Owner','email':'owner@example.test'}])
    def test_short_password_and_missing_database_do_not_create_or_reset_data(self):
        with self.assertRaises(ValueError):module.recover(self.db,{'action':'reset','email':'owner@example.test','password':'short'},hasher=lambda _: 'unused')
        missing=self.root/'missing.db'
        with self.assertRaises(ValueError):module.recover(missing,{'action':'list'})
        self.assertFalse(missing.exists())

if __name__=='__main__':unittest.main()
