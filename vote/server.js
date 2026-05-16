const express = require('express');
const fs = require('fs');
const path = require('path');

const app = express();
const PORT = process.env.PORT || 3210;
const DATA_FILE = path.join(__dirname, 'votes.json');

function loadVotes() {
  try {
    return JSON.parse(fs.readFileSync(DATA_FILE, 'utf8'));
  } catch {
    return {};
  }
}

function saveVotes(votes) {
  fs.writeFileSync(DATA_FILE, JSON.stringify(votes, null, 2));
}

app.use(express.json());
app.use(express.static(path.join(__dirname, 'public')));

app.get('/api/votes', (req, res) => {
  res.json(loadVotes());
});

app.post('/api/vote', (req, res) => {
  const { projectId } = req.body;
  if (!projectId) return res.status(400).json({ error: 'Missing projectId' });
  const votes = loadVotes();
  votes[projectId] = (votes[projectId] || 0) + 1;
  saveVotes(votes);
  res.json({ success: true, votes });
});

app.post('/api/unvote', (req, res) => {
  const { projectId } = req.body;
  if (!projectId) return res.status(400).json({ error: 'Missing projectId' });
  const votes = loadVotes();
  votes[projectId] = Math.max((votes[projectId] || 0) - 1, 0);
  saveVotes(votes);
  res.json({ success: true, votes });
});

app.listen(PORT, () => {
  console.log(`Vote server running on http://localhost:${PORT}`);
});
