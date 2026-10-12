package ai.storyteller.photocraft;

import android.app.NativeActivity;
import android.content.Intent;
import android.database.Cursor;
import android.net.Uri;
import android.os.Bundle;
import android.provider.OpenableColumns;
import org.json.JSONArray;
import org.json.JSONObject;
import java.io.InputStream;
import java.io.OutputStream;
import java.io.IOException;

/** Platform-only NativeActivity/SAF glue; document processing and UI stay in Rust. */
public class FileHelper extends NativeActivity {
    private static final int OPEN = 1001, SAVE = 1002;
    private String result;
    private final JSONArray incoming = new JSONArray();

    @Override protected void onCreate(Bundle state) {
        super.onCreate(state);
        receive(getIntent());
    }
    @Override protected void onNewIntent(Intent intent) {
        super.onNewIntent(intent);
        setIntent(intent);
        receive(intent);
    }
    private synchronized void receive(Intent intent) {
        if (intent == null) return;
        if (Intent.ACTION_VIEW.equals(intent.getAction())) {
            append(incoming, intent.getData());
        } else if (Intent.ACTION_SEND.equals(intent.getAction())) {
            append(incoming, intent.getParcelableExtra(Intent.EXTRA_STREAM));
        }
    }
    public synchronized String takeIncoming() {
        String value = incoming.toString();
        while (incoming.length() > 0) incoming.remove(incoming.length() - 1);
        return value;
    }
    public synchronized String pollResult() {
        String value = result;
        result = null;
        return value;
    }
    public void openFile(String mime, boolean multiple) {
        Intent intent = new Intent(Intent.ACTION_OPEN_DOCUMENT);
        intent.setType(mime);
        intent.addCategory(Intent.CATEGORY_OPENABLE);
        intent.putExtra(Intent.EXTRA_ALLOW_MULTIPLE, multiple);
        intent.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION | Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION);
        launch(intent, OPEN);
    }
    public void saveFile(String name) {
        Intent intent = new Intent(Intent.ACTION_CREATE_DOCUMENT);
        intent.setType("application/octet-stream");
        intent.addCategory(Intent.CATEGORY_OPENABLE);
        intent.putExtra(Intent.EXTRA_TITLE, name);
        intent.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION | Intent.FLAG_GRANT_WRITE_URI_PERMISSION | Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION);
        launch(intent, SAVE);
    }
    private void launch(Intent intent, int request) {
        synchronized (this) { result = null; }
        runOnUiThread(() -> {
            try { startActivityForResult(intent, request); }
            catch (RuntimeException e) { finishResult(new JSONArray(), e.toString()); }
        });
    }
    private synchronized void finishResult(JSONArray files, String error) {
        try {
            JSONObject value = new JSONObject().put("files", files);
            if (error != null) value.put("error", error);
            result = value.toString();
        } catch (org.json.JSONException e) { result = "{\"files\":[],\"error\":\"Invalid picker result\"}"; }
    }
    @Override protected void onActivityResult(int request, int code, Intent data) {
        super.onActivityResult(request, code, data);
        if (request != OPEN && request != SAVE) return;
        JSONArray files = new JSONArray();
        if (code == RESULT_OK && data != null) {
            if (data.getClipData() != null && request == OPEN) {
                for (int i = 0; i < data.getClipData().getItemCount(); i++) {
                    Uri uri = data.getClipData().getItemAt(i).getUri();
                    retain(uri, data.getFlags());
                    append(files, uri);
                }
            } else {
                retain(data.getData(), data.getFlags());
                append(files, data.getData());
            }
        }
        finishResult(files, null);
    }
    private void retain(Uri uri, int flags) {
        if (uri == null) return;
        int granted = flags & (Intent.FLAG_GRANT_READ_URI_PERMISSION | Intent.FLAG_GRANT_WRITE_URI_PERMISSION);
        try { getContentResolver().takePersistableUriPermission(uri, granted); }
        catch (SecurityException ignored) { /* Transient grants still permit this session. */ }
    }
    private void append(JSONArray files, Uri uri) {
        if (uri == null) return;
        String name = uri.getLastPathSegment();
        try (Cursor cursor = getContentResolver().query(uri, new String[]{OpenableColumns.DISPLAY_NAME}, null, null, null)) {
            if (cursor != null && cursor.moveToFirst()) name = cursor.getString(0);
        } catch (RuntimeException ignored) { }
        if (name == null || name.isEmpty()) name = "untitled";
        try { files.put(new JSONObject().put("name", name.replace('/', '_').replace('\\', '_')).put("uri", uri.toString())); }
        catch (org.json.JSONException ignored) { }
    }
    public InputStream openRead(String uri) throws IOException {
        return getContentResolver().openInputStream(Uri.parse(uri));
    }
    public OutputStream openWrite(String uri) throws IOException {
        return getContentResolver().openOutputStream(Uri.parse(uri), "wt");
    }
}
